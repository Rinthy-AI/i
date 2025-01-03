use std::collections::{HashMap, HashSet};

use crate::ast::{BinaryOp, IndexExpr, NoOp, ScalarOp, Schedule, Symbol, UnaryOp};
use crate::block::{Arg, Block, Expr, Statement, Type};
use crate::graph::{Graph, Node};

// NOTES:
// - Each interior node has bounds for its outputs named by its caller. It is
//   responsible for naming bounds for any remaining unnamed inputs.
// - Each leaf node is responsible for creating the declarations based on the
//   names it is given.

pub struct Lowerer {
    input_counter: usize,
    bound_counter: usize,
    iterator_counter: usize,
    store_counter: usize,
    split_factor_count: usize,
}

// TODO: Remove this
pub fn lower(graph: &Graph) -> Block {
    Lowerer::new().lower(graph)
}

impl Lowerer {
    pub fn new() -> Self {
        Lowerer {
            input_counter: 0,
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

        // TODO: output store should also be handled here since it needs to exist for
        //       current node to "call" (use) it

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

        self.lower_node(&graph.root, output_bound_idents, store_ident)
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

        // TODO: maybe create a statement to alias this? alternative is use store_ident in
        //       arg list
        //format!("in{}", self.input_counter)

        let statements =
            output_bound_idents
                .iter()
                .enumerate()
                .map(|(dim, ident)| Statement::Declaration {
                    ident: ident.clone(),
                    value: Expr::ArrayDim {
                        ident: store_ident.clone(),
                        dim: dim,
                    },
                    type_: Type::Int,
                });

        self.input_counter += 1;

        Block { statements: vec![] }
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

        // TODO: The mapping should probably be done in the present function instead of passing
        //       the hashmap here.
        let op_statement = Self::create_op_statement(
            op,
            &base_iterator_idents,
            &child_store_idents,
            &child_indices,
            store_ident,
            &index,
        );

        //let (char_index, rank) = schedule.loop_order[0];
        //println!(
        //    "{:#?}",
        //    (bound_idents[&char_index].clone(), base_iterator_idents[&char_index].clone(), rank)
        //);
        let loop_statements: Vec<Statement> =
            Self::create_empty_loop_statements(&schedule, &base_iterator_idents, &bound_idents);

        //println!("{:#?}", loop_statement);

        Block {
            statements: [
                split_factor_assignment_statements,
                loop_statements,
                vec![op_statement],
            ]
            .concat(),
        }
    }

    fn create_op_statement(
        op: &ScalarOp,
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
            index: index
                .chars()
                .map(|c| base_iterator_idents[&c].clone())
                .collect(),
        };

        let mut in_exprs: Vec<Expr> = child_store_idents
            .iter()
            .zip(child_indices.iter())
            .map(|(ident, index)| Expr::Indexed {
                ident: ident.clone(),
                index: index
                    .chars()
                    .map(|c| base_iterator_idents[&c].clone())
                    .collect(),
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
    ) -> Vec<Statement> {
        // for each loop in loop order:
        //     is it a base loop?
        //     is it a factor loop?
        //     does it require index reconstruction?

        vec![Statement::Loop {
            index: "todo".to_string(),
            bound: "todo".to_string(),
            body: Block { statements: vec![] },
        }]
    }
}
