use crate::ast::{
    BinaryOp, Combinator, Expr, ExprBank, ExprRef, IndexExpr, NoOp, ScalarOp, Schedule, UnaryOp,
};

#[derive(Clone, Debug)]
pub enum Node {
    Leaf {
        index: String,
    },
    Interior {
        index: String,
        op: char,
        children: Vec<(Node, String)>, // the child node and the index according to current `Node`
        schedule: Schedule,
    },
}

impl Node {
    pub fn get_leaves_mut(&mut self) -> Vec<&mut Node> {
        match self {
            Node::Leaf { .. } => vec![self],
            Node::Interior { children, .. } => children
                .iter_mut()
                .flat_map(|(child, _index)| child.get_leaves_mut())
                .collect(),
        }
    }

    pub fn index(&self) -> String {
        match self {
            Self::Leaf { index, .. } | Self::Interior { index, .. } => index.to_string(),
        }
    }

    pub fn children(&self) -> Option<&Vec<(Node, String)>> {
        match self {
            Node::Leaf { .. } => None,
            Node::Interior { children, .. } => Some(&children),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Graph {
    nodes: Vec<Node>,
}

impl Graph {
    pub fn new(nodes: Vec<Node>) -> Self {
        Self { nodes }
    }

    pub fn root(&self) -> &Node {
        &self.nodes[0]
    }
}

fn node(expr_ref: &ExprRef, expr_bank: &ExprBank) -> Node {
    let Some(expr) = &expr_bank.0.get(expr_ref.0) else {
        panic!("Expression Bank is empty.")
    };
    match expr {
        Expr::Index(IndexExpr { op, out, schedule }) => Node::Interior {
            index: out.0.clone(),
            op: match op {
                ScalarOp::UnaryOp(UnaryOp::Accum(_)) | ScalarOp::BinaryOp(BinaryOp::Add(_, _)) => {
                    '+'
                }
                ScalarOp::UnaryOp(UnaryOp::Prod(_)) | ScalarOp::BinaryOp(BinaryOp::Mul(_, _)) => {
                    '*'
                }
                ScalarOp::NoOp(_) => ' ', // never used
            },
            children: match op {
                ScalarOp::BinaryOp(BinaryOp::Add(in0, in1))
                | ScalarOp::BinaryOp(BinaryOp::Mul(in0, in1)) => vec![
                    (
                        Node::Leaf {
                            index: in0.0.clone(),
                        },
                        in0.0.clone(),
                    ),
                    (
                        Node::Leaf {
                            index: in1.0.clone(),
                        },
                        in1.0.clone(),
                    ),
                ],
                ScalarOp::UnaryOp(UnaryOp::Accum(in0)) | ScalarOp::UnaryOp(UnaryOp::Prod(in0)) => {
                    vec![(
                        Node::Leaf {
                            index: in0.0.clone(),
                        },
                        in0.0.clone(),
                    )]
                }
                ScalarOp::NoOp(NoOp(in0)) => vec![(
                    Node::Leaf {
                        index: in0.0.clone(),
                    },
                    in0.0.clone(),
                )],
            },
            schedule: schedule.clone(),
        },
        Expr::Combinator(combinator) => match combinator {
            Combinator::Chain(left_ref, right_ref) => {
                let left = node(left_ref, expr_bank);
                let mut right = node(right_ref, expr_bank);
                if let Node::Interior { .. } = right {
                    if let Some(first) = right.get_leaves_mut().first_mut() {
                        **first = left;
                        right
                    } else {
                        panic!("Right expr in `Chain` has no children.")
                    }
                } else {
                    panic!("Right expr in `Chain` is a leaf node.")
                }
            }
        },
    }
}

pub fn graph(expr_bank: &ExprBank) -> Graph {
    Graph::new(vec![node(&ExprRef(expr_bank.0.len() - 1), &expr_bank)])
}
