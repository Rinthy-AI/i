use std::sync::{Arc, Mutex};

use crate::ast::{
    BinaryOp, Combinator, Expr, ExprBank, ExprRef, IndexExpr, NoOp, ScalarOp, Schedule, UnaryOp,
};

type NodeRef = Arc<Mutex<Node>>;

#[derive(Clone, Debug)]
pub enum NodeBody {
    Leaf,
    Interior { op: char, schedule: Schedule },
}

#[derive(Clone, Debug)]
pub struct Node {
    pub index: String,
    pub body: NodeBody,
    pub parents: Vec<NodeRef>,
    children: Vec<(NodeRef, String)>, // child node and its index according to the current Node
}

impl Node {
    pub fn children(&self) -> Vec<(Node, String)> {
        self.children
            .iter()
            .map(|(child_ref, index)| (child_ref.lock().unwrap().clone(), index.clone()))
            .collect()
    }
}

fn get_leftmost_parent_of_leaf(node: &NodeRef) -> Option<NodeRef> {
    let mut current = Arc::clone(node);
    let mut parent = None;

    loop {
        let next = {
            let node = current.lock().unwrap();
            if node.children.is_empty() {
                return parent;
            }
            Arc::clone(&node.children[0].0)
        };
        parent = Some(current);
        current = next;
    }
}

fn get_leftmost_leaf(node_ref: &NodeRef) -> NodeRef {
    let mut current = Arc::clone(node_ref);
    loop {
        let next = {
            let node = current.lock().unwrap();
            if node.children.is_empty() {
                return Arc::clone(&current);
            }
            Arc::clone(&node.children[0].0)
        };
        current = next;
    }
}

#[derive(Clone, Debug)]
pub struct Graph {
    nodes: Vec<NodeRef>,
}

impl Graph {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn root(&self) -> NodeRef {
        Arc::clone(self.nodes.last().expect("Graph is empty"))
    }

    pub fn from_expr_bank(expr_bank: &ExprBank) -> Graph {
        let mut graph = Self::new();
        graph.from_expr_ref_with_expr_bank(&ExprRef(expr_bank.0.len() - 1), expr_bank, vec![]);
        graph
    }

    /// Return the `Graph` formed by chaining `self` into `other`, i.e., `f.chain(g) = g(f)`
    /// Equivalent to the commutation of `compose`: `f.chain(g) = g.compose(f)`
    pub fn chain(&self, other: &Self) -> Self {
        let right = other.clone();
        let left = self.clone();
        if let Some(parent) = get_leftmost_parent_of_leaf(&right.root()) {
            let mut parent_node = parent.lock().unwrap();
            parent_node.children[0] = (left.root().clone(), parent_node.children[0].1.to_string());
        }
        right
    }

    /// Return the `Graph` formed by composing `self` with `other`, i.e., `f.compose(g) = f(g)`
    /// Equivalent to the commutation of `chain`: `f.compose(g) = g.chain(f)`
    pub fn compose(&self, other: &Self) -> Self {
        other.chain(self)
    }

    fn add_node(
        &mut self,
        index: String,
        body: NodeBody,
        parents: Vec<NodeRef>,
        children: Vec<(NodeRef, String)>,
    ) -> NodeRef {
        let node = Arc::new(Mutex::new(Node {
            index: index.clone(),
            body,
            parents: parents.clone(),
            children,
        }));

        for p in parents {
            p.lock()
                .unwrap()
                .children
                .push((Arc::clone(&node), index.clone()));
        }

        self.nodes.push(Arc::clone(&node));
        node
    }

    fn from_expr_ref_with_expr_bank(
        &mut self,
        expr_ref: &ExprRef,
        expr_bank: &ExprBank,
        parents: Vec<NodeRef>,
    ) -> NodeRef {
        let Some(expr) = &expr_bank.0.get(expr_ref.0) else {
            panic!("Expression Bank is empty.")
        };

        match expr {
            Expr::Index(IndexExpr { op, out, schedule }) => {
                let children = match op {
                    ScalarOp::BinaryOp(BinaryOp::Add(in0, in1))
                    | ScalarOp::BinaryOp(BinaryOp::Mul(in0, in1))
                    | ScalarOp::BinaryOp(BinaryOp::Max(in0, in1)) => vec![
                        (
                            self.add_node(in0.0.clone(), NodeBody::Leaf, vec![], vec![]),
                            in0.0.clone(),
                        ),
                        (
                            self.add_node(in1.0.clone(), NodeBody::Leaf, vec![], vec![]),
                            in1.0.clone(),
                        ),
                    ],
                    ScalarOp::UnaryOp(UnaryOp::Accum(in0))
                    | ScalarOp::UnaryOp(UnaryOp::Prod(in0))
                    | ScalarOp::UnaryOp(UnaryOp::Relu(in0))
                    | ScalarOp::NoOp(NoOp(in0)) => {
                        vec![(
                            self.add_node(in0.0.clone(), NodeBody::Leaf, vec![], vec![]),
                            in0.0.clone(),
                        )]
                    }
                };
                let op = match op {
                    ScalarOp::UnaryOp(UnaryOp::Accum(_))
                    | ScalarOp::BinaryOp(BinaryOp::Add(_, _)) => '+',
                    ScalarOp::UnaryOp(UnaryOp::Prod(_))
                    | ScalarOp::BinaryOp(BinaryOp::Mul(_, _)) => '*',
                    ScalarOp::UnaryOp(UnaryOp::Relu(_))
                    | ScalarOp::BinaryOp(BinaryOp::Max(_, _)) => '>',
                    ScalarOp::NoOp(_) => ' ', // never used
                };
                let body = NodeBody::Interior {
                    op,
                    schedule: schedule.clone(),
                };
                self.add_node(out.0.clone(), body, parents, children)
            }
            Expr::Combinator(Combinator::Chain(left_ref, right_ref)) => {
                let left = self.from_expr_ref_with_expr_bank(left_ref, expr_bank, parents.clone());
                let right = self.from_expr_ref_with_expr_bank(right_ref, expr_bank, parents);
                if let Some(parent) = get_leftmost_parent_of_leaf(&right) {
                    let mut parent_node = parent.lock().unwrap();
                    parent_node.children[0] =
                        (Arc::clone(&left), parent_node.children[0].1.to_string());
                }
                right
            }
        }
    }
}
