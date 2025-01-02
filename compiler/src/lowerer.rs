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
        }
    }

    fn get_char_indices(index: &String) -> Vec<char> {
        let mut indices: Vec<char> = index.chars().collect::<HashSet<_>>().into_iter().collect();
        indices.sort();
        indices
    }

    pub fn lower(&mut self, graph: &Graph) -> Block {
        let indices = Self::get_char_indices(&graph.root.index());

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

        self.lower_node(&graph.root)
    }

    fn lower_node(&mut self, node: &Node) -> Block {
        match node {
            Node::Leaf { .. } => self.lower_leaf_node(node, &vec![]),
            Node::Interior { .. } => self.lower_interior_node(node),
        }
    }

    fn lower_leaf_node(&mut self, node: &Node, bound_idents: &Vec<(String, usize)>) -> Block {
        let Node::Leaf { index } = node else {
            panic!("Expected leaf node.")
        };

        let statements =
            bound_idents
                .iter()
                .enumerate()
                .map(|(ind, (ident, dim))| Statement::Declaration {
                    ident: ident.clone(),
                    value: Expr::ArrayDim {
                        ident: format!("in{}", ind + self.input_counter),
                        dim: *dim,
                    },
                    type_: Type::Int,
                });

        self.input_counter += bound_idents.len();

        Block { statements: vec![] }
    }

    fn lower_interior_node(&mut self, node: &Node) -> Block {
        let Node::Interior {
            index,
            op,
            children,
            schedule,
        } = node
        else {
            panic!("Expected interior node.")
        };

        let indexes = [
            children
                .iter()
                .map(|child| child.index())
                .collect::<Vec<_>>(),
            vec![index.to_string()],
        ]
        .concat();

        let mut indices = indexes
            .iter()
            .flat_map(|index| index.chars().map(|c| c.to_string()))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        indices.sort();

        // create idents for bounds, base iterators (splits to be affixed with `_{ind}`), and store
        let mut bound_idents = HashMap::<String, String>::new();
        let mut base_iterator_idents = HashMap::<String, String>::new();
        for (ind, index) in indices.iter().enumerate() {
            bound_idents.insert(index.clone(), format!("b{}", ind + self.bound_counter));
            base_iterator_idents.insert(index.clone(), format!("i{}", ind + self.iterator_counter));
        }
        let store_ident = format!("s{}", self.store_counter);
        self.bound_counter += bound_idents.len();
        self.iterator_counter += base_iterator_idents.len();
        self.store_counter += 1;

        // determine splits

        Block { statements: vec![] }
    }
}
