use crate::ast::{
    BinaryOp, Combinator, Expr, ExprBank, ExprRef, IndexExpr, NoOp, ScalarOp, UnaryOp,
};
use crate::graph::{Graph, Node};
use crate::parser;

impl Node {
    fn from_i(src: &str) -> Result<Self, String> {
        let mut parser = parser::Parser::new(src).unwrap();
        let expr = parser.parse_index_expr().unwrap();
        Ok(Self::from_index_expr(&expr))
    }

    fn from_index_expr(expr: &IndexExpr) -> Self {
        let IndexExpr { op, out, schedule } = expr;
        let child_indices = match op {
            ScalarOp::BinaryOp(BinaryOp::Add(in0, in1))
            | ScalarOp::BinaryOp(BinaryOp::Mul(in0, in1)) => vec![in0.0.clone(), in1.0.clone()],
            ScalarOp::UnaryOp(UnaryOp::Accum(in0))
            | ScalarOp::UnaryOp(UnaryOp::Prod(in0))
            | ScalarOp::NoOp(NoOp(in0)) => vec![in0.0.clone()],
        };
        Node::Interior {
            index: out.0.clone(),
            child_indices: child_indices.clone(),
            op: op.clone(),
            children: child_indices
                .into_iter()
                .map(|index| Node::Leaf { index })
                .collect(),
            schedule: schedule.clone(),
        }
    }

    fn from_expr_ref_and_expr_bank(expr_ref: &ExprRef, expr_bank: &ExprBank) -> Node {
        let Some(expr) = &expr_bank.0.get(expr_ref.0) else {
            panic!("Expression Bank is empty.")
        };
        match expr {
            Expr::Index(expr) => Self::from_index_expr(expr),
            Expr::Combinator(combinator) => match combinator {
                Combinator::Chain(left_ref, right_ref) => {
                    let mut left = Node::from_expr_ref_and_expr_bank(left_ref, expr_bank);
                    let mut right = Node::from_expr_ref_and_expr_bank(right_ref, expr_bank);
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
}

impl Graph {
    pub fn from_i(src: &str) -> Result<Self, String> {
        let mut parser = parser::Parser::new(src).unwrap();
        let expr = parser.parse_index_expr().unwrap();
        Ok(Graph {
            root: Node::from_index_expr(&expr),
        })
    }
}

pub fn graph(expr_bank: &ExprBank) -> Graph {
    Graph {
        root: Node::from_expr_ref_and_expr_bank(&ExprRef(expr_bank.0.len() - 1), &expr_bank),
    }
}
