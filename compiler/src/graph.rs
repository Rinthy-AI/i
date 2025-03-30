use std::cell::RefCell;
use std::rc::Rc;

use crate::ast::{
    BinaryOp, Combinator, Expr, ExprBank, ExprRef, IndexExpr, NoOp, ScalarOp, Schedule, UnaryOp,
};

type NodeRef = Rc<RefCell<Node>>;

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
            .map(|(child_ref, index)| (child_ref.borrow().clone(), index.clone()))
            .collect()
    }
}

fn get_leftmost_parent_of_leaf(node: &NodeRef) -> Option<NodeRef> {
    let mut current = node.clone();
    let mut parent = None;

    loop {
        let next = {
            let node = current.borrow();
            if node.children.is_empty() {
                return parent;
            }
            node.children[0].0.clone()
        };
        parent = Some(current);
        current = next;
    }
}

fn get_leftmost_leaf(node_ref: &NodeRef) -> NodeRef {
    let mut current = node_ref.clone();
    loop {
        let next = {
            let node = current.borrow();
            if node.children.is_empty() {
                return current.clone();
            }
            node.children[0].0.clone()
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
        self.nodes.last().expect("Graph is empty").clone()
    }

    pub fn add_node(
        &mut self,
        index: String,
        body: NodeBody,
        parents: Vec<NodeRef>,
        children: Vec<(NodeRef, String)>,
    ) -> NodeRef {
        let node = Rc::new(RefCell::new(Node {
            index: index.clone(),
            body,
            parents: parents.clone(),
            children,
        }));

        for p in parents {
            p.borrow_mut()
                .children
                .push((Rc::clone(&node), index.clone()));
        }

        self.nodes.push(Rc::clone(&node));
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
                    | ScalarOp::BinaryOp(BinaryOp::Mul(in0, in1)) => vec![
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
                    let mut parent_node = parent.borrow_mut();
                    parent_node.children[0] =
                        (Rc::clone(&left), parent_node.children[0].1.to_string());
                }
                right
            }
        }
    }

    pub fn from_expr_bank(expr_bank: &ExprBank) -> Graph {
        let mut graph = Self::new();
        graph.from_expr_ref_with_expr_bank(&ExprRef(expr_bank.0.len() - 1), expr_bank, vec![]);
        graph
    }
}
