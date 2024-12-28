use std::collections::{HashMap, HashSet};

use crate::ast::{BinaryOp, IndexExpr, NoOp, ScalarOp, Schedule, Symbol, UnaryOp};
use crate::block::{Access, Alloc, ArrayDim, Block, Loop, Value};

pub fn lower(dep: &IndexExpr) -> Block {
    let IndexExpr {
        op: scalar_op,
        out: result_index,
        schedule: Schedule { splits, loop_order },
    } = dep;

    let mut split_counter: HashMap<String, usize> = splits
        .iter()
        .map(|(dim, split)| (dim.clone(), split.len()))
        .collect();

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
    for index in &indices {
        // insert index itself
        values.insert(index.clone(), Value::Index(index.clone()));

        // get iterator bound from index, e.g., `i` -> `ni`
        let bound = format!("n{index}");
        // TODO: anywhere `flattened.len()>1` can become an assert
        let flattened = input_index_vecs
            .iter()
            .enumerate()
            .flat_map(|(input_ind, input_index_vec)| {
                input_index_vec
                    .iter()
                    .enumerate()
                    .filter(|(_, ch)| *ch == index)
                    .map(move |(dim, _)| (input_ind, dim))
            })
            .collect::<Vec<_>>();

        let (input, dim) = flattened[0]; // TODO: What if this fails?
        values.insert(bound, Value::ArrayDim(ArrayDim { input, dim }));
        if let Some(split_factors) = splits.get(index) {
            for (ind, factor) in split_factors.iter().enumerate() {
                values.insert(format!("n{index}{ind}"), Value::Uint(*factor));
            }
        }
    }

    #[derive(Clone, Debug)]
    enum LoopIndex {
        Base(String),
        Split(String, i32),
    }

    let mut loop_indices = Vec::new();
    for (index, rank) in loop_order {
        if *rank == 0 {
            loop_indices.push(LoopIndex::Base(index.clone()));
        } else {
            loop_indices.push(LoopIndex::Split(
                index.clone(),
                splits[index][*rank as usize - 1],
            ));
        }
    }

    if loop_order.is_empty() {
        loop_indices = indices
            .iter()
            .map(|index| LoopIndex::Base(index.clone()))
            .collect();
    }

    // TODO: Clean this up
    let loops = loop_indices
        .iter()
        .map(|index| {
            match index {
                LoopIndex::Base(index) => {
                    let mut bound = format!("n{index}");
                    if let Some(loop_splits) = splits.get(index) {
                        let n_loop_splits = loop_splits.len();
                        let tile_width_string = format!(
                            "({})",
                            (0..n_loop_splits)
                                .map(|i| format!("n{index}{i}"))
                                .collect::<Vec<_>>()
                                .join(" * ")
                        );
                        bound = format!("({bound} + {tile_width_string} - 1)/{tile_width_string}");
                    }

                    Loop {
                        index: index.to_string(),
                        bound,
                        index_reconstruction: None,
                    }
                }
                LoopIndex::Split(base_index, factor) => {
                    // TODO: handle index_reconstruction
                    let n_loop_splits_total = splits
                        .get(base_index)
                        .expect("Could not find expected loop splits")
                        .len();

                    let n_loop_splits_remaining = split_counter
                        .get_mut(base_index)
                        .expect("Could not find expected loop split count");

                    let n_loop_splits_processed = n_loop_splits_total - *n_loop_splits_remaining;

                    *n_loop_splits_remaining -= 1;

                    let index = format!("{base_index}{n_loop_splits_processed}");

                    let index_reconstruction_string = if *n_loop_splits_remaining == 0 {

                        // TODO: This is also computed in the above match arm. Can we dedup?
                        let tile_width_string = format!(
                            "({})",
                            (0..n_loop_splits_total)
                                .map(|i| format!("n{base_index}{i}"))
                                .collect::<Vec<_>>()
                                .join(" * ")
                        );

                        let interim_loop_element_width_strings = (0..n_loop_splits_total - 1)
                            .map(|ind| format!(" + n{base_index}{ind} * {base_index}{ind}"))
                            .collect::<Vec<_>>()
                            .join("");

                        Some(format!(
                            "{base_index} * {tile_width_string}{interim_loop_element_width_strings} + {base_index}{}",
                            n_loop_splits_total - 1
                        ))
                    } else {
                        None
                    };

                    Loop {
                        index: index.clone(),
                        bound: format!("n{index}"),
                        index_reconstruction: match index_reconstruction_string {
                            Some(index_reconstruction_string) =>
                                Some((base_index.clone(), index_reconstruction_string)),
                            None => None,
                        }
                    }
                },
            }
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
            schedule: _,
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
            ScalarOp::NoOp(NoOp(in0_index)) => (vec![in0_index.array_index_strings()], '+', 0.0),
        }
    }
}

impl Symbol {
    fn array_index_strings(&self) -> Vec<String> {
        self.0.chars().map(|c| c.to_string()).collect()
    }
}
