use std::collections::{HashMap, HashSet};

use crate::ast::{BinaryOp, IndexExpr, NoOp, ScalarOp, Schedule, Symbol, UnaryOp};
use crate::block::{Arg, Block, Expr, Statement, Type};
use crate::graph::{Graph, Node};

pub struct Lowerer {
    input_idents: Vec<String>,
    input_args: Vec<Arg>,
    bound_counter: usize,
    iterator_counter: usize,
    store_counter: usize,
    split_factor_count: usize,
}

impl Lowerer {
    pub fn new() -> Self {
        Lowerer {
            input_idents: Vec::new(),
            input_args: Vec::new(),
            bound_counter: 0,
            iterator_counter: 0,
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
        let indices = Self::get_char_indices(&graph.root.index());

        // create ident for store
        let store_ident = format!("out");

        let mut bound_idents = HashMap::<char, String>::new();
        for (ind, index) in indices.iter().enumerate() {
            bound_idents.insert(*index, format!("b{}", ind + self.bound_counter));
        }
        self.bound_counter += bound_idents.len();

        let output_bound_idents: Vec<String> = graph
            .root
            .index()
            .chars()
            .map(|c| bound_idents[&c].clone())
            .collect();

        let nodes_block = self.lower_node(&graph.root, output_bound_idents.clone(), store_ident);

        let (output_array_args, dim_arg_declarations) = self.create_args_and_ident_declarations(
            "out".to_string(),
            output_bound_idents.clone(),
            true,
        );

        self.input_args.extend(output_array_args);

        let mut function_statement = Statement::Function {
            ident: "f".to_string(),
            type_: Type::Array,
            args: self.input_args.clone(),
            body: Block {
                statements: [dim_arg_declarations, nodes_block.statements].concat(),
            },
        };

        Block {
            statements: vec![function_statement],
        }
    }

    fn lower_node(
        &mut self,
        node: &Node,
        output_bound_idents: Vec<String>,
        store_ident: String,
    ) -> Block {
        match node {
            Node::Leaf { .. } => self.lower_leaf_node(node, output_bound_idents, store_ident),
            Node::Interior { .. } => {
                self.lower_interior_node(node, output_bound_idents, store_ident)
            }
        }
    }

    fn lower_leaf_node(
        &mut self,
        node: &Node,
        output_bound_idents: Vec<String>,
        store_ident: String,
    ) -> Block {
        let Node::Leaf { index } = node else {
            panic!("Expected leaf node.")
        };

        self.input_idents.push(store_ident.clone());

        // push array arg
        self.input_args.push(Arg {
            type_: Type::Array,
            ident: store_ident.clone(),
            mutable: false,
        });

        // push dim args and create declaration statements
        let mut statements = vec![];
        for (ind, bound_ident) in output_bound_idents.iter().enumerate() {
            let arg_ident = format!("{}_{}", store_ident.clone(), ind);

            // map nice dim arg names to messy generated bound idents
            statements.push(Statement::Declaration {
                ident: bound_ident.clone(),
                value: Expr::Ident(arg_ident.clone()),
                type_: Type::Int,
            });

            self.input_args.push(Arg {
                type_: Type::Int,
                ident: arg_ident,
                mutable: false,
            });
        }

        Block { statements }
    }

    fn lower_interior_node(
        &mut self,
        node: &Node,
        output_bound_idents: Vec<String>,
        store_ident: String,
    ) -> Block {
        let Node::Interior {
            index,
            op,
            children,
            schedule,
        } = node
        else {
            panic!("Expected interior node.")
        };

        // insert output_bound_idents into table first
        let output_char_indices = Self::get_char_indices(&node.index());
        let mut bound_idents: HashMap<char, String> = output_char_indices
            .iter()
            .zip(output_bound_idents.iter())
            .map(|(char_index, bound_ident)| (*char_index, bound_ident.clone()))
            .collect();

        let child_indices: Vec<String> = children.iter().map(|child| child.index()).collect();

        // create and insert input bound idents into table (not overwriting existing idents)
        for (ind, char_index) in child_indices
            .iter()
            .flat_map(|index| index.chars().collect::<Vec<_>>())
            .enumerate()
        {
            bound_idents.entry(char_index).or_insert_with(|| {
                let ident = format!("b{}", self.bound_counter);
                self.bound_counter += 1;
                ident
            });
        }

        let mut all_char_indices: Vec<char> = bound_idents.keys().map(|c| *c).collect();
        all_char_indices.sort();

        let mut schedule = schedule.clone(); // Can we avoid this?

        if schedule.loop_order.is_empty() {
            schedule.loop_order = all_char_indices
                .iter()
                .map(|index| (index.clone(), 0))
                .collect();
        }

        // create idents for base iterators (splits to be affixed with `_{ind}`)
        let base_iterator_idents: HashMap<char, String> = all_char_indices
            .iter()
            .enumerate()
            .map(|(ind, char_index)| (*char_index, format!("i{}", ind + self.iterator_counter)))
            .collect();
        self.iterator_counter += base_iterator_idents.len();

        // create store ident for each child
        let child_store_idents: Vec<String> = children
            .iter()
            .enumerate()
            .map(|(ind, child)| format!("s{}", ind + self.store_counter))
            .collect();
        self.store_counter += child_store_idents.len();

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
                                bound_idents[char_index], self.split_factor_count
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
                        type_: Type::Int,
                    })
            })
            .collect();

        let alloc_statement = Statement::Declaration {
            ident: store_ident.clone(),
            value: Expr::Alloc {
                initial_value: 0.,
                //shape: index.chars().map(|c| output_bound_idents[&c].clone()).collect(),
                shape: output_bound_idents.clone(),
            },
            type_: Type::Array,
        };

        // TODO: The mapping should probably be done in the present function instead of passing
        //       the hashmap here.
        let op_statement = Self::create_op_statement(
            op,
            &bound_idents,
            &base_iterator_idents,
            &child_store_idents,
            &child_indices,
            store_ident,
            &index,
        );

        let loop_statements: Vec<Statement> = Self::create_empty_loop_statements(
            &schedule,
            &base_iterator_idents,
            &bound_idents,
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

        let child_statements: Vec<Statement> = children
            .iter()
            .enumerate()
            .flat_map(|(ind, child)| {
                let child_block = self.lower_node(
                    &child,
                    child
                        .index()
                        .chars()
                        .map(|c| bound_idents[&c].clone())
                        .collect(),
                    child_store_idents[ind].clone(),
                );
                child_block.statements
            })
            .collect();

        Block {
            statements: [
                child_statements,
                split_factor_assignment_statements,
                vec![alloc_statement, loop_stack],
            ]
            .concat(),
        }
    }

    fn create_op_statement(
        op: &ScalarOp,
        bound_idents: &HashMap<char, String>,
        base_iterator_idents: &HashMap<char, String>,
        child_store_idents: &Vec<String>,
        child_indices: &Vec<String>,
        store_ident: String,
        index: &String,
    ) -> Statement {
        assert_eq!(child_store_idents.len(), child_indices.len());

        let op_char = match op {
            ScalarOp::UnaryOp(UnaryOp::Accum(_)) | ScalarOp::BinaryOp(BinaryOp::Add(_, _)) => '+',
            ScalarOp::UnaryOp(UnaryOp::Prod(_)) | ScalarOp::BinaryOp(BinaryOp::Mul(_, _)) => '*',
            ScalarOp::NoOp(_) => ' ', // never used
        };

        let out_expr = Expr::Indexed {
            ident: store_ident,
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
                type_: Type::Int,
            },
            Statement::Skip {
                index: base_iterator_ident.clone(),
                bound: base_bound_ident.clone(),
            },
        ]
    }

    fn create_affine_index(indices: Vec<String>, bounds: Vec<String>) -> Expr {
        let d = indices.len();
        let mut sum_expr = Expr::Int(0);
        for k in 0..d {
            let mut product_expr = Expr::Int(1);
            for m in (k + 1)..d {
                product_expr = Expr::Op {
                    op: '*',
                    inputs: vec![product_expr, Expr::Ident(bounds[m].clone())],
                };
            }
            let partial_expr = Expr::Op {
                op: '*',
                inputs: vec![Expr::Ident(indices[k].clone()), product_expr],
            };
            sum_expr = Expr::Op {
                op: '+',
                inputs: vec![sum_expr, partial_expr],
            };
        }
        sum_expr
    }

    fn create_args_and_ident_declarations(
        &mut self,
        ident: String,
        bound_idents: Vec<String>,
        mutable: bool,
    ) -> (Vec<Arg>, Vec<Statement>) {
        let mut args = Vec::new();

        // push array arg
        args.push(Arg {
            type_: Type::Array,
            ident: ident.clone(),
            mutable: mutable,
        });

        // push dim args and create declaration statements
        let mut statements = vec![];
        for (ind, bound_ident) in bound_idents.iter().enumerate() {
            let dim_arg_ident = format!("{}_{}", ident.clone(), ind);

            args.push(Arg {
                type_: Type::Int,
                ident: dim_arg_ident.clone(),
                mutable: false,
            });

            // map nice dim arg names to messy generated bound idents
            statements.push(Statement::Declaration {
                ident: bound_ident.clone(),
                value: Expr::Ident(dim_arg_ident),
                type_: Type::Int,
            });
        }

        (args, statements)
    }
}
