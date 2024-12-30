use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct Loop {
    pub index: String, // ident of Value::Uint iterator, e.g., `i`
    pub bound: String, // ident of Value::ArrayDim, e.g., `ni`
    pub index_reconstruction: Option<(String, String)>, // ident of Value
}

#[derive(Clone, Debug)]
pub enum Expr {
    Alloc {
        initial_value: f32,
        shape: Vec<String>,
    },
    ArrayDim {
        input: usize,
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
        ident: String,
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
}

#[derive(Clone, Debug)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub loops: Vec<Loop>,
    pub op: Statement,
    pub splits: HashMap<String, Vec<String>>, // from arraydim value to uint values
}
