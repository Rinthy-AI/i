use std::collections::{HashMap, HashSet};

use crate::ir::{ BinaryOp, Dependency, NoOp, ScalarOp, Symbol, UnaryOp, };

#[derive(Clone, Debug)]
pub struct Loop {
    pub iterations: String, // ident of Value::ArrayDim, e.g., `ni`
    pub index_reconstruction: Option<String>, // ident of Value
}

#[derive(Clone, Debug)]
pub struct Alloc {
    pub initial_value: f32,
    pub shape: Vec<String>,
    pub index: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Access {
    pub indices: Vec<String>, // `Variable` ident, one per dim of accessed Array
}

#[derive(Clone, Debug)]
pub struct ArrayDim {
    pub input: usize,
    pub dim: usize,
}

#[derive(Clone, Debug)]
pub enum Value {
    ArrayDim(ArrayDim), // size of array dimension, e.g.,  `ni`
    Index(String), // an index variable, e.g., `i`
    Uint(i32),
}

#[derive(Clone, Debug)]
pub struct Node {
    pub alloc: Alloc,
    pub accesses: Vec<Access>,
    pub loops: Vec<Loop>,
    pub op: char, // this can't be a char forever
    pub values: HashMap<String, Value>,
    pub splits: HashMap<String, Vec<String>>, // from arraydim value to uint values
}

impl Node {
    pub fn new(dep: &Dependency) -> Node {
        let Dependency{ op: scalar_op, out: result_index } = dep;

        let (
            input_index_vecs,
            output_index_vec,
            op,
            initial_value
        ) = dep.get_index_vecs_op_char_and_init_value();

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
            values.insert(bound, Value::ArrayDim(ArrayDim{input, dim}));
        }

        let loops = indices
            .iter()
            .map(|index| Loop {
                iterations: format!("n{index}"),
                index_reconstruction: None,
            })
            .collect();

        Node {
            alloc,
            accesses,
            op,
            loops,
            values,
            splits: HashMap::new(),
        }
    }
}

impl Dependency {
    /// Returns index vec for each input, index vec for output, op char
    fn get_index_vecs_op_char_and_init_value(&self) -> (
        Vec<Vec<String>>, Vec<String>, char, f32
    ) {
        let Dependency{ op: scalar_op, out: output_index } = self;
        let (input_index_vec, op_char, init_value) = scalar_op.get_index_vecs_op_char_and_init_value();
        (input_index_vec, output_index.array_index_strings(), op_char, init_value)
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
            ScalarOp::NoOp(NoOp(in0_index)) => {
                (vec![in0_index.array_index_strings()], '+', 1.0)
            }
        }
    }
}

impl Symbol {
    fn array_index_strings(&self) -> Vec<String> {
        self.0.chars().map(|c| c.to_string()).collect()
    }
}
