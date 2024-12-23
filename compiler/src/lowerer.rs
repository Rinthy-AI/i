use std::collections::{HashMap, HashSet};

use crate::ast::{BinaryOp, IndexExpr, NoOp, ScalarOp, Symbol, UnaryOp};
use crate::block::{Loop, Alloc, Access, ArrayDim, Value, Block};

pub fn lower(dep: &IndexExpr) -> Block {
    let IndexExpr {
        op: scalar_op,
        out: result_index,
    } = dep;

    let (input_index_vecs, output_index_vec, op, initial_value) =
        dep.get_index_vecs_op_char_and_init_value();

    let alloc = Alloc {
        initial_value,
        shape: output_index_vec.iter().map(|c| format!("n{c}")).collect(),
        index: output_index_vec.clone(),
    };

    let accesses = input_index_vecs
        .iter()
        .map(|indices| Access {
            indices: indices.clone(),
        })
        .collect();

    let indices: Vec<String> = input_index_vecs
        .iter()
        .flat_map(|v| v.iter())
        .chain(output_index_vec.iter())
        .flat_map(|s| s.chars())
        .collect::<HashSet<_>>()
        .into_iter()
        .map(|c| c.to_string())
        .collect();

    let mut values = HashMap::new();
    for ind in &indices {
        // insert index itself
        values.insert(ind.clone(), Value::Index(ind.clone()));

        // get iterator bound from index, e.g., `i` -> `ni`
        let bound = format!("n{ind}");
        // TODO: anywhere `flattened.len()>1` can become an assert
        let flattened = input_index_vecs
            .iter()
            .enumerate()
            .flat_map(|(input_ind, input_index_vec)| {
                input_index_vec
                    .iter()
                    .enumerate()
                    .filter(|(_, ch)| *ch == ind)
                    .map(move |(dim, _)| (input_ind, dim))
            })
            .collect::<Vec<_>>();

        let (input, dim) = flattened[0]; // TODO: What if this fails?
        values.insert(bound, Value::ArrayDim(ArrayDim { input, dim }));
    }

    let loops = indices
        .iter()
        .map(|index| Loop {
            bound: format!("n{index}"),
            index_reconstruction: None,
        })
        .collect();

    Block {
        alloc,
        accesses,
        op,
        loops,
        values,
        splits: HashMap::new(),
    }
}

impl IndexExpr {
    /// Returns index vec for each input, index vec for output, op char
    fn get_index_vecs_op_char_and_init_value(&self) -> (Vec<Vec<String>>, Vec<String>, char, f32) {
        let IndexExpr {
            op: scalar_op,
            out: output_index,
        } = self;
        let (input_index_vec, op_char, init_value) =
            scalar_op.get_index_vecs_op_char_and_init_value();
        (
            input_index_vec,
            output_index.array_index_strings(),
            op_char,
            init_value,
        )
    }
}

impl ScalarOp {
    /// Returns index vec for each input and the op char
    fn get_index_vecs_op_char_and_init_value(&self) -> (Vec<Vec<String>>, char, f32) {
        match self {
            ScalarOp::BinaryOp(BinaryOp::Mul(in0_index, in1_index)) => (
                vec![
                    in0_index.array_index_strings(),
                    in1_index.array_index_strings(),
                ],
                '*',
                1.0,
            ),
            ScalarOp::BinaryOp(BinaryOp::Add(in0_index, in1_index)) => (
                vec![
                    in0_index.array_index_strings(),
                    in1_index.array_index_strings(),
                ],
                '+',
                0.0,
            ),
            ScalarOp::UnaryOp(UnaryOp::Prod(in0_index)) => {
                (vec![in0_index.array_index_strings()], '*', 1.0)
            }
            ScalarOp::UnaryOp(UnaryOp::Accum(in0_index)) => {
                (vec![in0_index.array_index_strings()], '+', 0.0)
            }
            ScalarOp::NoOp(NoOp(in0_index)) => (vec![in0_index.array_index_strings()], '+', 1.0),
        }
    }
}

impl Symbol {
    fn array_index_strings(&self) -> Vec<String> {
        self.0.chars().map(|c| c.to_string()).collect()
    }
}
