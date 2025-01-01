use std::collections::{HashMap, HashSet};

use crate::ast::{BinaryOp, IndexExpr, NoOp, ScalarOp, Schedule, Symbol, UnaryOp};
use crate::block::{Arg, Block, Expr, Statement, Type};
use crate::graph::{ Graph, Node };

pub fn lower(graph: &Graph) -> Block {
    let Graph { root: node } = graph;

    let mut block = lower_node(node);
    block.statements.push(Statement::Return { value: Expr::Ident("out".to_string()) });

    Block {
        statements: vec![
            Statement::Function{
                ident: "f".to_string(),
                type_: Type::Array,
                args: (0..graph.get_leaves().len())
                    .map(|ind| Arg { type_: Type::Array, ident: format!("in{ind}")})
                    .collect(),
                body: block,
            }
        ],
    }
}

fn lower_node(node: &Node) -> Block {
    let Node::Interior {
        index: result_index,
        op: scalar_op,
        children,
        schedule: Schedule { splits, loop_order },
    } = node
    else { panic!("Root node in graph was not of variant `Interior`") };

    let (input_index_vecs, op, initial_value) =
        scalar_op.get_index_vecs_op_char_and_init_value();
    let output_index_vec = split_index_string(&result_index);

    let mut statements = Vec::new();

    let indices: Vec<String> = input_index_vecs
        .iter()
        .flat_map(|v| v.iter())
        .chain(output_index_vec.iter())
        .flat_map(|s| s.chars())
        .collect::<HashSet<_>>()
        .into_iter()
        .map(|c| c.to_string())
        .collect();

    let loop_order = if loop_order.is_empty() {
        &indices.iter().map(|index| (index.clone(), 0)).collect()
    } else {
        loop_order
    };

    let mut splits = splits.clone();
    for index in &indices {
        splits.entry(index.clone()).or_insert_with(Vec::new);
    }

    // for counting the loop splits processed so far
    let mut split_counter: HashMap<String, usize> =
        splits.iter().map(|(dim, split)| (dim.clone(), 0)).collect();

    let indexed_in_arrays: Vec<_> = input_index_vecs
        .iter()
        .enumerate()
        .map(|(ind, index)| Expr::Indexed {
            ident: format!("in{ind}"),
            index: index.clone(),
        })
        .collect();

    for index in &indices {
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

        let (input_ind, dim) = flattened[0]; // TODO: What if this fails?
        statements.push(Statement::Declaration {
            ident: bound,
            value: Expr::ArrayDim {
                ident: format!("in{input_ind}"),
                dim,
            },
            type_: Type::Int,
        });
        if let Some(split_factors) = splits.get(index) {
            for (ind, factor) in split_factors.iter().enumerate() {
                statements.push(Statement::Declaration {
                    ident: format!("n{index}{ind}"),
                    value: Expr::Int(*factor),
                    type_: Type::Int,
                });
            }
        }
    }

    statements.push(Statement::Declaration {
        ident: "out".to_string(),
        value: Expr::Alloc {
            initial_value,
            shape: output_index_vec.iter().map(|c| format!("n{c}")).collect(),
        },
        type_: Type::Array,
    });

    let indexed_out_expr = Expr::Indexed {
        ident: "out".to_string(),
        index: output_index_vec,
    };

    let partial_op_expr = Expr::Op {
        op: op,
        inputs: indexed_in_arrays,
    };

    let op = Statement::Assignment {
        left: indexed_out_expr.clone(),
        right: Expr::Op {
            op: op,
            inputs: vec![indexed_out_expr, partial_op_expr],
        },
    };

    let loop_stack = loop_order
        .iter()
        .rev()
        .map(|(index, rank)| {
            let (base_index, index, bound) = match rank {
                0 => {
                    let mut bound = format!("n{index}");
                    let loop_splits = &splits[index];
                    if loop_splits.len() > 0 {
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
                    (index.to_string(), index.to_string(), bound)
                }
                _ => {
                    let base_index = index.clone();
                    let index = format!("{base_index}{}", rank - 1);

                    (base_index.to_string(), index.to_string(), format!("n{index}"))
                },

            };

            *split_counter
                .get_mut(&base_index)
                .expect("Could not find expected loop split count") += 1;

            // index reconstruction logic, to be performed on innermost loop of a split "family"
            let n_index_family_loops = splits[&base_index].len() + 1;
            let body = if split_counter[&base_index] == 1 && n_index_family_loops > 1 {
                let n_loop_splits_total = splits
                    .get(&base_index)
                    .expect("Could not find expected loop splits")
                    .len();

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

                vec![
                    Statement::Declaration {
                        ident: base_index.clone(),
                        value: Expr::Str(format!(
                            "{base_index} * {tile_width_string}{interim_loop_element_width_strings} + {base_index}{}",
                            n_loop_splits_total - 1
                        )),
                        type_: Type::Int,
                    },
                    Statement::Skip {
                        index: base_index.clone(),
                        bound: format!("n{}", base_index.clone())
                    },
                ]
            } else {
                vec![]
            };

            Statement::Loop {
                index,
                bound,
                body: Block {
                    statements: body,
                }
            }
        })
        .fold(op.clone(), |mut loop_stack, mut loop_| {
            if let Statement::Loop{ ref mut body, .. } = loop_ {
                body.statements.push(loop_stack);
            }
            loop_
        });

    statements.push(loop_stack);

    Block {
        statements,
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

fn split_index_string(s: &String) -> Vec<String> {
    s.chars().map(|c| c.to_string()).collect()
}
