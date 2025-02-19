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
    Int(usize),
    Ident(String),
    Op {
        op: char,
        inputs: Vec<Expr>,
    },
    Indexed {
        ident: String, // TODO: Should be Expr (Ident)
        index: Box<Expr>,
    },
}

// Should this be an Expr variant?
#[derive(Clone, Debug)]
pub struct Arg {
    pub type_: Type,
    pub ident: String,
}

#[derive(Clone, Debug)]
pub enum Type {
    Int(bool),
    Array(bool),
    ArrayRef(bool),
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
        type_: Type,
    },
    Skip {
        // TODO: These should both probably be Expr (Ident)
        index: String,
        bound: String,
    },
    Loop {
        index: String,
        bound: Expr,
        body: Block,
        parallel: bool,
    },
    Return {
        value: Expr,
    },
    Function {
        ident: String,
        args: Vec<Arg>, // type, ident
        body: Block,
    },
}

#[derive(Clone, Debug)]
pub struct Block {
    pub statements: Vec<Statement>,
}
