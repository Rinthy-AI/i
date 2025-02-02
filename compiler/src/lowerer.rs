use std::collections::{HashMap, HashSet};

use crate::ast::Schedule;
use crate::block::{Arg, Block, Expr, Statement, Type};
use crate::graph::{Graph, Node};

pub struct Lowerer {
    input_args: Vec<Arg>,
    input_array_counter: usize,
    base_loop_counter: usize,
    store_counter: usize,
    split_factor_count: usize,
}

impl Lowerer {
    pub fn new() -> Self {
        Lowerer {
            input_args: Vec::new(),
            input_array_counter: 0,
            base_loop_counter: 0,
            store_counter: 0,
            split_factor_count: 0,
        }
    }

    fn get_char_indices(index: &String) -> Vec<char> {
        let mut indices: Vec<char> = index.chars().collect::<HashSet<_>>().into_iter().collect();
        indices.sort();
        indices
    }

    pub fn lower(&mut self, graph: &Graph) -> Block {
        let (def_block, exec_block, _loop_idents, _store_ident) =
            self.lower_node(&graph.root, true);
        Block {
            statements: [
                def_block.statements,
                vec![Statement::Function {
                    ident: "f".to_string(),
                    args: self.input_args.clone(),
                    body: Block {
                        statements: exec_block.statements,
                    },
                }],
            ]
            .concat(),
        }
    }

    /// Return function def block, exec block, (bound, iterator) ident map, store ident
    fn lower_node(
        &mut self,
        node: &Node,
        root: bool,
    ) -> (Block, Block, HashMap<char, (String, String)>, String) {
        match node {
            Node::Leaf { index, .. } => self.lower_leaf_node(&index),
            Node::Interior {
                index,
                op,
                children,
                schedule,
            } => self.lower_interior_node(index, op, children, schedule, root),
        }
    }

    /// Return function def block, exec block, (bound, iterator) ident map, store ident
    fn lower_leaf_node(
        &mut self,
        index: &String,
    ) -> (Block, Block, HashMap<char, (String, String)>, String) {
        let arg_ident = format!("in{}", self.input_array_counter);
        self.input_array_counter += 1;

        let char_indices = Self::get_char_indices(index);

        let loop_idents: HashMap<char, (String, String)> = char_indices
            .iter()
            .map(|char_index| {
                let bound_ident = format!("b{}", self.base_loop_counter);
                let iterator_ident = format!("i{}", self.base_loop_counter);
                self.base_loop_counter += 1;
                (*char_index, (bound_ident, iterator_ident))
            })
            .collect();

        // push array arg
        self.input_args.push(Arg {
            type_: Type::ArrayRef(false),
            ident: Expr::Ident(arg_ident.clone()),
        });

        // push dim args
        let dim_args = char_indices.iter().map(|c| Arg {
            type_: Type::Int(false),
            ident: Expr::Ident(loop_idents[c].0.clone()),
        });
        self.input_args.extend(dim_args.clone());

        (Block::EMPTY, Block::EMPTY, loop_idents, arg_ident)
    }

    /// Return function definition block, exec block, (bound, iterator) ident map, and store ident
    fn lower_interior_node(
        &mut self,
        index: &String,
        op: &char,
        children: &Vec<(Node, String)>,
        schedule: &Schedule,
        root: bool,
    ) -> (Block, Block, HashMap<char, (String, String)>, String) {
        let (child_def_block, child_exec_block, loop_idents, child_store_idents): (
            Block,
            Block,
            HashMap<char, (String, String)>,
            Vec<String>,
        ) = children.iter().fold(
            (Block::EMPTY, Block::EMPTY, HashMap::new(), vec![]),
            |(mut def_block, mut exec_block, mut loop_idents, mut child_store_idents),
             (child, index)| {
                let (child_def_block, child_exec_block, child_loop_idents, child_store_ident) =
                    self.lower_node(&child, false);

                // for mapping between child indexing and current node indexing
                let index_map: HashMap<char, char> =
                    child.index().chars().zip(index.chars()).collect();

                let child_loop_idents: HashMap<char, (String, String)> = child_loop_idents
                    .into_iter()
                    .map(|(c, x)| (*index_map.get(&c).unwrap_or(&c), x))
                    .filter(|(c, _)| !loop_idents.contains_key(c))
                    .collect();

                def_block.statements.extend(child_def_block.statements);
                exec_block.statements.extend(child_exec_block.statements);
                loop_idents.extend(child_loop_idents);
                child_store_idents.push(child_store_ident);
                (def_block, exec_block, loop_idents, child_store_idents)
            },
        );

        let store_ident = match root {
            true => "out".to_string(),
            false => {
                let ident = format!("s{}", self.store_counter);
                self.store_counter += 1;
                ident
            }
        };

        let mut all_char_indices: Vec<char> = loop_idents.keys().map(|c| *c).collect();
        all_char_indices.sort();

        let mut schedule = schedule.clone(); // Can we avoid this?
        if schedule.loop_order.is_empty() {
            schedule.loop_order = all_char_indices
                .iter()
                .map(|index| (index.clone(), 0))
                .collect();
        }

        // create split factor idents
        let split_factor_idents: HashMap<char, Vec<String>> = schedule
            .splits
            .iter()
            .map(|(char_index, split_list)| {
                (
                    *char_index,
                    split_list
                        .iter()
                        .enumerate()
                        .map(|(ind, _split_factor)| {
                            let split_factor_ident = format!(
                                "{}_{ind}_{}",
                                loop_idents[char_index].0, self.split_factor_count
                            );
                            self.split_factor_count += 1;
                            split_factor_ident
                        })
                        .collect(),
                )
            })
            .collect();

        // create assignment statement for each split factor ident
        let split_factor_assignment_statements: Vec<Statement> = schedule
            .splits
            .iter()
            .flat_map(|(char_index, split_factors)| {
                split_factors
                    .iter()
                    .zip(split_factor_idents[char_index].iter())
                    .map(|(factor, ident)| Statement::Declaration {
                        ident: ident.clone(),
                        value: Expr::Int(*factor),
                        type_: Type::Int(false),
                    })
            })
            .collect();

        let alloc_statement = Statement::Declaration {
            ident: store_ident.clone(),
            value: Expr::Alloc {
                initial_value: 0.,
                shape: index.chars().map(|c| loop_idents[&c].0.clone()).collect(),
            },
            type_: Type::Array(true),
        };

        // TODO: The mapping should probably be done in the present function instead of passing
        //       the hashmap here.
        // TODO: stop splitting ident map
        let op_statement = Self::create_op_statement(
            op,
            // bound_idents
            &loop_idents
                .iter()
                .map(|(c, (ident, _))| (*c, ident.clone()))
                .collect(),
            // base_iterator_idents
            &loop_idents
                .iter()
                .map(|(c, (_, ident))| (*c, ident.clone()))
                .collect(),
            &child_store_idents,
            &children
                .iter()
                .map(|(child, index)| index.clone())
                .collect(),
            &store_ident,
            &index,
        );

        // TODO: stop splitting ident map
        let loop_statements: Vec<Statement> = Self::create_empty_loop_statements(
            &schedule,
            &loop_idents
                .iter()
                .map(|(c, (_, ident))| (*c, ident.clone()))
                .collect(),
            &loop_idents
                .iter()
                .map(|(c, (ident, _))| (*c, ident.clone()))
                .collect(),
            &split_factor_idents,
            &index,
        );

        let loop_stack: Statement =
            loop_statements
                .into_iter()
                .fold(op_statement, |loop_stack, mut loop_| {
                    if let Statement::Loop { ref mut body, .. } = loop_ {
                        body.statements.push(loop_stack);
                    }
                    loop_
                });

        if root {
            // push array arg
            self.input_args.push(Arg {
                type_: Type::ArrayRef(true),
                ident: Expr::Ident(store_ident.clone()),
            });

            // push dim args
            let dim_args = (0..Self::get_char_indices(&index).len()).map(|ind| Arg {
                type_: Type::Int(false),
                ident: Expr::Ident(format!("{}_{ind}", store_ident.clone())),
            });
            self.input_args.extend(dim_args.clone());
        };

        let function_ident = format!("_{}", store_ident.clone());

        let def_block = Block {
            statements: [
                child_def_block.statements,
                vec![Statement::Function {
                    ident: function_ident.clone(),
                    args: [
                        child_store_idents
                            .iter()
                            .map(|ident| Arg {
                                type_: Type::ArrayRef(false),
                                ident: Expr::Ident(ident.clone()),
                            })
                            .collect::<Vec<_>>(),
                        vec![Arg {
                            type_: Type::ArrayRef(true),
                            ident: Expr::Ident(store_ident.clone()),
                        }],
                        all_char_indices
                            .iter()
                            .map(|c| Arg {
                                type_: Type::Int(false),
                                ident: Expr::Ident(loop_idents[c].0.clone()),
                            })
                            .collect::<Vec<_>>(),
                    ]
                    .concat(),
                    body: Block {
                        statements: [split_factor_assignment_statements, vec![loop_stack]].concat(),
                    },
                }],
            ]
            .concat(),
        };

        let call = Statement::Call {
            ident: function_ident.clone(),
            args: [
                child_store_idents
                    .iter()
                    .map(|ident| Arg {
                        type_: Type::ArrayRef(false),
                        ident: Expr::Ref(ident.clone(), false),
                    })
                    .collect::<Vec<_>>(),
                vec![Arg {
                    type_: Type::ArrayRef(true),
                    ident: Expr::Ref(store_ident.clone(), true),
                }],
                all_char_indices
                    .iter()
                    .map(|c| Arg {
                        type_: Type::Int(false),
                        ident: Expr::Ident(loop_idents[c].0.clone()),
                    })
                    .collect::<Vec<_>>(),
            ]
            .concat(),
        };

        let exec_block = Block {
            statements: [
                child_exec_block.statements,
                if root { vec![] } else { vec![alloc_statement] }, // TODO: Make not hacky.
                vec![call],
            ]
            .concat(),
        };

        (def_block, exec_block, loop_idents, store_ident)
    }

    fn create_op_statement(
        op: &char,
        bound_idents: &HashMap<char, String>,
        base_iterator_idents: &HashMap<char, String>,
        child_store_idents: &Vec<String>,
        child_indices: &Vec<String>,
        store_ident: &String,
        index: &String,
    ) -> Statement {
        assert_eq!(child_store_idents.len(), child_indices.len());

        let out_expr = Expr::Indexed {
            ident: store_ident.clone(),
            index: Box::new(Self::create_affine_index(
                index
                    .chars()
                    .map(|c| base_iterator_idents[&c].clone())
                    .collect(),
                index.chars().map(|c| bound_idents[&c].clone()).collect(),
            )),
        };

        let mut in_exprs: Vec<Expr> = child_store_idents
            .iter()
            .zip(child_indices.iter())
            .map(|(ident, index)| Expr::Indexed {
                ident: ident.clone(),
                index: Box::new(Self::create_affine_index(
                    index
                        .chars()
                        .map(|c| base_iterator_idents[&c].clone())
                        .collect(),
                    index.chars().map(|c| bound_idents[&c].clone()).collect(),
                )),
            })
            .collect();

        if in_exprs.len() == 1 {
            // Pushing to front here shouldn't be a problem unless we start allowing ops of
            // arbitrary inputs.
            in_exprs.insert(0, out_expr.clone());
        }
        assert_eq!(
            in_exprs.len(),
            2,
            "Expected exactly two operands for op [{op}]."
        );

        Statement::Assignment {
            left: out_expr,
            right: Expr::Op {
                op: *op,
                inputs: in_exprs,
            },
        }
    }

    fn create_empty_loop_statements(
        schedule: &Schedule,
        base_iterator_idents: &HashMap<char, String>,
        bound_idents: &HashMap<char, String>,
        split_factor_idents: &HashMap<char, Vec<String>>,
        index: &String,
    ) -> Vec<Statement> {
        let mut statements = vec![];

        let mut needs_index_reconstruction: HashSet<char> = schedule
            .splits
            .iter()
            .filter(|(_c, splits_factors)| splits_factors.len() > 0)
            .map(|(c, _splits_factors)| *c)
            .collect();

        let output_char_indices: HashSet<char> = index.chars().collect();
        for (char_index, rank) in schedule.loop_order.iter().rev() {
            let splits = schedule.splits.get(char_index);

            let index = if splits.is_some() && *rank > 0 {
                format!(
                    "{}_{}",
                    base_iterator_idents[&char_index].clone(),
                    (*rank - 1)
                )
            } else {
                base_iterator_idents[&char_index].clone()
            };

            let bound = match (splits, rank) {
                (None, _) => Expr::Ident(bound_idents[&char_index].clone()),
                (Some(_splits), 0) => Self::create_split_bound_expr(
                    &bound_idents[&char_index],
                    &split_factor_idents[&char_index],
                ),
                (Some(_splits), rank) => {
                    Expr::Ident(split_factor_idents[&char_index][*rank - 1].clone())
                }
            };

            statements.push(Statement::Loop {
                index: index.clone(),
                bound: bound,
                body: Block {
                    statements: if needs_index_reconstruction.remove(&char_index) {
                        Self::create_index_reconstruction_statements(
                            &base_iterator_idents[&char_index],
                            &bound_idents[&char_index],
                            &split_factor_idents[&char_index],
                            *rank,
                        )
                    } else {
                        vec![]
                    },
                },
                parallel: output_char_indices.contains(&char_index),
            });
        }

        statements
    }

    fn create_split_bound_expr(
        base_bound_ident: &String,
        split_factors_idents: &Vec<String>,
    ) -> Expr {
        let tile_width_expr = Expr::Op {
            op: '*',
            inputs: split_factors_idents
                .iter()
                .map(|ident| Expr::Ident(ident.clone()))
                .collect(),
        };

        let numerator = Expr::Op {
            op: '-',
            inputs: vec![
                Expr::Op {
                    op: '+',
                    inputs: vec![
                        Expr::Ident(base_bound_ident.clone()),
                        tile_width_expr.clone(),
                    ],
                },
                Expr::Int(1),
            ],
        };

        Expr::Op {
            op: '/',
            inputs: vec![numerator, tile_width_expr],
        }
    }

    fn create_index_reconstruction_statements(
        base_iterator_ident: &String,
        base_bound_ident: &String,
        split_factors_idents: &Vec<String>,
        rank: usize,
    ) -> Vec<Statement> {
        let factor_loop_widths: Vec<Expr> = split_factors_idents
            .iter()
            .map(|ident| Expr::Ident(ident.clone()))
            .collect();

        // number of elements per iteration of base loop
        let base_loop_tile_width = Expr::Op {
            op: '*',
            inputs: factor_loop_widths.clone(),
        };

        let mut widths = factor_loop_widths;
        widths.insert(0, base_loop_tile_width);

        let factor_loop_iterator: Vec<Expr> = (0..split_factors_idents.len())
            .map(|ind| Expr::Ident(format!("{}_{ind}", base_iterator_ident.clone())))
            .collect();

        let mut iterators = factor_loop_iterator;
        iterators.insert(0, Expr::Ident(base_iterator_ident.clone()));

        // remove present loop before total width calculation
        let current_iterator = iterators.remove(rank);
        widths.remove(rank);

        assert_eq!(widths.len(), iterators.len());
        let mut total_width: Vec<Expr> = widths
            .into_iter()
            .zip(iterators.into_iter())
            .map(|(width, iterator)| Expr::Op {
                op: '*',
                inputs: vec![width, iterator],
            })
            .collect();

        total_width.push(current_iterator);

        let reconstructed_index = Expr::Op {
            op: '+',
            inputs: total_width,
        };

        vec![
            Statement::Declaration {
                ident: base_iterator_ident.clone(),
                value: reconstructed_index,
                type_: Type::Int(false),
            },
            Statement::Skip {
                index: base_iterator_ident.clone(),
                bound: base_bound_ident.clone(),
            },
        ]
    }

    fn create_affine_index(indices: Vec<String>, bounds: Vec<String>) -> Expr {
        let d = indices.len();
        let mut sum_expr = None;
        for k in 0..d {
            let mut product_expr = None;
            for m in (k + 1)..d {
                product_expr = Some(match product_expr {
                    Some(expr) => Expr::Op {
                        op: '*',
                        inputs: vec![expr, Expr::Ident(bounds[m].clone())],
                    },
                    None => Expr::Ident(bounds[m].clone()),
                });
            }
            let partial_expr = match product_expr {
                Some(expr) => Expr::Op {
                    op: '*',
                    inputs: vec![Expr::Ident(indices[k].clone()), expr],
                },
                None => Expr::Ident(indices[k].clone()),
            };
            sum_expr = Some(match sum_expr {
                Some(expr) => Expr::Op {
                    op: '+',
                    inputs: vec![expr, partial_expr],
                },
                None => partial_expr,
            });
        }
        sum_expr.unwrap_or(Expr::Int(0)) // Return 0 if no indices are provided
    }
}
