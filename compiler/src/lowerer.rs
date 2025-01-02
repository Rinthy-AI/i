use std::collections::{HashMap, HashSet};

use crate::ast::{BinaryOp, IndexExpr, NoOp, ScalarOp, Schedule, Symbol, UnaryOp};
use crate::block::{Arg, Block, Expr, Statement, Type};
use crate::graph::{Graph, Node};

pub struct Lowerer {
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
            bound_counter: 0,
            iterator_counter: 0,
            store_counter: 0,
        }
    }

    pub fn lower(&mut self, graph: &Graph) -> Block {
        //println!("{:#?}", graph);
        let mut bound_counter = 0;
        let mut iterator_counter = 0;
        let mut store_counter = 0;
        self.lower_node(&graph.root, bound_counter, iterator_counter, store_counter)
    }

    fn lower_node(
        &mut self,
        node: &Node,
        mut bound_counter: usize,
        mut iterator_counter: usize,
        mut store_counter: usize,
    ) -> Block {
        match node {
            Node::Leaf { .. } => {
                self.lower_leaf_node(node, bound_counter, iterator_counter, store_counter)
            }
            Node::Interior { .. } => {
                self.lower_interior_node(node, bound_counter, iterator_counter, store_counter)
            }
        }
    }

    fn lower_leaf_node(
        &mut self,
        node: &Node,
        mut bound_counter: usize,
        mut iterator_counter: usize,
        mut store_counter: usize,
    ) -> Block {
        let Node::Leaf { index } = node else {
            panic!("Expected leaf node.")
        };

        Block { statements: vec![] }
    }

    fn lower_interior_node(
        &mut self,
        node: &Node,
        mut bound_counter: usize,
        mut iterator_counter: usize,
        mut store_counter: usize,
    ) -> Block {
        // NOTES:
        // - Each node determines the bounds for its input indices only (?)

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
            bound_idents.insert(index.clone(), format!("b{}", ind + bound_counter));
            base_iterator_idents.insert(index.clone(), format!("i{}", ind + bound_counter));
        }
        let store_ident = format!("s{store_counter}");
        bound_counter += bound_idents.len();
        iterator_counter += base_iterator_idents.len();
        store_counter += 1;

        println!("{:#?}", bound_idents);
        println!("{:#?}", base_iterator_idents);
        println!("{:#?}", store_ident);

        // determine splits

        Block { statements: vec![] }
    }
}
