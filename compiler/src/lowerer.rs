use std::collections::{HashMap, HashSet};

use crate::ast::{BinaryOp, IndexExpr, NoOp, ScalarOp, Schedule, Symbol, UnaryOp};
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
        let (block, bound_and_iterator_idents, store_ident) = self.lower_node(&graph.root, true);
        Block {
            statements: vec![Statement::Function {
                ident: "f".to_string(),
                args: self.input_args.clone(),
                body: Block {
                    statements: block.statements,
                },
            }],
        }
    }

    /// Return the block, (bound, iterator) ident map, and store ident
    fn lower_node(
        &mut self,
        node: &Node,
        root: bool,
    ) -> (Block, HashMap<char, (String, String)>, String) {
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

    fn lower_leaf_node(
        &mut self,
        index: &String,
    ) -> (Block, HashMap<char, (String, String)>, String) {
        let arg_ident = format!("in{}", self.input_array_counter);
        self.input_array_counter += 1;

        let char_indices = Self::get_char_indices(index);

        let loop_idents: HashMap<_, _> = char_indices
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
            ident: arg_ident.clone(),
        });

        // push dim args
        let dim_args = char_indices.iter().map(|c| Arg {
            type_: Type::Int(false),
            ident: loop_idents[c].0.clone(),
        });
        self.input_args.extend(dim_args.clone());

        (Block::EMPTY, loop_idents, arg_ident)
    }

    fn lower_interior_node(
        &mut self,
        index: &String,
        op: &ScalarOp,
        children: &Vec<Node>,
        schedule: &Schedule,
        root: bool,
    ) -> (Block, HashMap<char, (String, String)>, String) {
        let (child_block, loop_idents, store_idents): (
            Block,
            HashMap<char, (String, String)>,
            Vec<String>,
        ) = children.iter().fold(
            (Block { statements: vec![] }, HashMap::new(), vec![]),
            |(mut block, mut loop_idents, mut store_idents), child| {
                let (child_block, mut child_loop_idents, child_store_ident) =
                    self.lower_node(&child, false);
                let child_loop_idents: HashMap<_, _> = child_loop_idents
                    .into_iter()
                    .filter(|(c, _)| !loop_idents.contains_key(c))
                    .collect();

                block.statements.extend(child_block.statements);
                loop_idents.extend(child_loop_idents);
                store_idents.push(child_store_ident);
                (block, loop_idents, store_idents)
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
            &store_idents,
            &children.iter().map(|child| child.index()).collect(),
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
                .fold(op_statement, |mut loop_stack, mut loop_| {
                    if let Statement::Loop { ref mut body, .. } = loop_ {
                        body.statements.push(loop_stack);
                    }
                    loop_
                });

        if root {
            // push array arg
            self.input_args.push(Arg {
                type_: Type::ArrayRef(true),
                ident: store_ident.clone(),
            });

            // push dim args
            let dim_args = (0..Self::get_char_indices(&index).len()).map(|ind| Arg {
                type_: Type::Int(false),
                ident: format!("{}_{ind}", store_ident.clone()),
            });
            self.input_args.extend(dim_args.clone());
        };

        let block = Block {
            statements: [
                child_block.statements,
                split_factor_assignment_statements,
                if root { vec![] } else { vec![alloc_statement] }, // TODO: Make not hacky.
                vec![loop_stack],
            ]
            .concat(),
        };

        (block, loop_idents, store_ident)
    }

    fn create_op_statement(
        op: &ScalarOp,
        bound_idents: &HashMap<char, String>,
        base_iterator_idents: &HashMap<char, String>,
        child_store_idents: &Vec<String>,
        child_indices: &Vec<String>,
        store_ident: &String,
        index: &String,
    ) -> Statement {
        assert_eq!(child_store_idents.len(), child_indices.len());

        let op_char = match op {
            ScalarOp::UnaryOp(UnaryOp::Accum(_)) | ScalarOp::BinaryOp(BinaryOp::Add(_, _)) => '+',
            ScalarOp::UnaryOp(UnaryOp::Prod(_)) | ScalarOp::BinaryOp(BinaryOp::Mul(_, _)) => '*',
            ScalarOp::NoOp(_) => ' ', // never used
        };

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
            "Expected exactly two operands for op [{op_char}]."
        );

        Statement::Assignment {
            left: out_expr,
            right: Expr::Op {
                op: op_char,
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
            .filter(|(char_index, splits_factors)| splits_factors.len() > 0)
            .map(|(char_index, splits_factors)| *char_index)
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
                (Some(splits), 0) => Self::create_split_bound_expr(
                    &bound_idents[&char_index],
                    &split_factor_idents[&char_index],
                ),
                (Some(splits), rank) => {
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
        let mut factor_loop_widths: Vec<Expr> = split_factors_idents
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

        let mut factor_loop_iterator: Vec<Expr> = split_factors_idents
            .iter()
            .enumerate()
            .map(|(ind, ident)| Expr::Ident(format!("{}_{ind}", base_iterator_ident.clone())))
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
