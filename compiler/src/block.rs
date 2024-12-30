use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub enum Expr {
    Alloc {
        initial_value: f32,
        shape: Vec<String>,
    },
    ArrayDim {
        ident: String, // TODO: Should be Expr (Ident)
        dim: usize,
    },
    Str(String),
    Int(i32),
    Ident(String),
    Op {
        op: char,
        inputs: Vec<Expr>,
    },
    Indexed {
        ident: String, // TODO: Should be Expr (Ident)
        index: Vec<String>,
    }
}

#[derive(Clone, Debug)]
pub enum Statement {
    Assignment {
        left: Expr, // Should LValue become it's own enum?
        right: Expr,
    },
    Declaration {
        ident: String,
        value: Expr,
    },
    Skip {
        // TODO: These should both probably be Expr (Ident)
        index: String,
        bound: String,
    },
    Loop {
        index: String,
        bound: String,
        body: Vec<Statement>,
    },
}

#[derive(Clone, Debug)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub loops: Vec<Statement>,
}
