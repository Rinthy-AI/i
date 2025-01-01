use crate::ast::{
    BinaryOp, Combinator, Expr, ExprBank, ExprRef, IndexExpr, NoOp, ScalarOp, UnaryOp
};
use crate::graph::{ Graph, Node };

fn node(expr_ref: &ExprRef, expr_bank: &ExprBank) -> Node {
    let Some(expr) = &expr_bank.0.get(expr_ref.0) else { panic!("Expression Bank is empty.") };
    match expr {
        Expr::Index(IndexExpr { op, out, schedule }) => {
            Node::Interior {
                index: out.0.clone(),
                op: match op {
                    ScalarOp::BinaryOp(BinaryOp::Add(_, _))
                    |ScalarOp::UnaryOp(UnaryOp::Accum(_))
                    |ScalarOp::NoOp(NoOp(_))
                    => '+',
                    ScalarOp::BinaryOp(BinaryOp::Mul(_, _))
                    | ScalarOp::UnaryOp(UnaryOp::Prod(_))
                    => '*',
                },
                children: match op {
                    ScalarOp::BinaryOp(BinaryOp::Add(in0, in1))
                    | ScalarOp::BinaryOp(BinaryOp::Mul(in0, in1)) => vec![
                        Node::Leaf { index: in0.0.clone() },
                        Node::Leaf { index: in1.0.clone() },
                    ],
                    ScalarOp::UnaryOp(UnaryOp::Accum(in0))
                    | ScalarOp::UnaryOp(UnaryOp::Prod(in0)) => vec![
                        Node::Leaf { index: in0.0.clone() },
                    ],
                    ScalarOp::NoOp(NoOp(in0)) => vec![Node::Leaf { index: in0.0.clone() }],
                }
            }
        }
        Expr::Combinator(combinator) => match combinator {
            Combinator::Chain(left_ref, right_ref) => {
                let mut left = node(left_ref, expr_bank);
                let mut right = node(right_ref, expr_bank);
                if let Node::Interior{ .. } = right {
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
        }
    }
}

pub fn graph(expr_bank: &ExprBank) -> Graph {
    Graph {
        root: node(&ExprRef(expr_bank.0.len() - 1), &expr_bank),
    }
}
