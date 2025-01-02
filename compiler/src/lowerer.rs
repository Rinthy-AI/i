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

        // create and insert input bound idents into table (not overwriting existing idents)
        for (ind, char_index) in children
            .iter()
            .map(|child| child.index())
            .flat_map(|index| index.chars().collect::<Vec<_>>())
            .enumerate()
        {
            bound_idents.entry(char_index).or_insert_with(|| {
                let ident = format!("b{}", self.bound_counter);
                self.bound_counter += 1;
                ident
            });
        }

        let indices: Vec<String> = vec![]; // TODO: remove

        // create idents for base iterators (splits to be affixed with `_{ind}`)
        let base_iterator_idents: HashMap<char, String> = bound_idents
            .keys()
            .enumerate()
            .map(|(ind, char_index)| (*char_index, format!("i{}", ind + self.iterator_counter)))
            .collect();
        self.iterator_counter += base_iterator_idents.len();

        // create store ident for each child
        let store_idents: Vec<String> = children
            .iter()
            .enumerate()
            .map(|(ind, child)| format!("s{}", ind + self.store_counter))
            .collect();
        self.store_counter += store_idents.len();

        // determine splits

        //let Self::create_op_statement(op, );

        println!("{:#?}", store_idents);

        Block { statements: vec![] }
    }
}
