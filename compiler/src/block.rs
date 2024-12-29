use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct Loop {
    pub index: String, // ident of Value::Uint iterator, e.g., `i`
    pub bound: String, // ident of Value::ArrayDim, e.g., `ni`
    pub index_reconstruction: Option<(String, String)>, // ident of Value
}

#[derive(Clone, Debug)]
pub struct Access {
    pub indices: Vec<String>, // `Variable` ident, one per dim of accessed Array
}

#[derive(Clone, Debug)]
pub struct ArrayDim {
    pub input: usize,
    pub dim: usize,
}

#[derive(Clone, Debug)]
pub enum Value {
    ArrayDim(ArrayDim), // size of array dimension, e.g., `ni`
    Index(String),      // an index variable, e.g., `i`
    Uint(i32),
}

#[derive(Clone, Debug)]
pub enum Expr {
    Alloc {
        initial_value: f32,
        shape: Vec<String>,
        index: Vec<String>,
    },
    ArrayDim {
        input: usize,
        dim: usize,
    },
    Str(String),
    Int(i32),
    Op {
        op: char,
        inputs: Vec<Expr>,
    }
}

#[derive(Clone, Debug)]
pub enum Statement {
    Declaration {
        ident: String,
        value: Expr,
    }
}

#[derive(Clone, Debug)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub accesses: Vec<Access>,
    pub loops: Vec<Loop>,
    pub op: char, // this can't be a char forever
    pub values: HashMap<String, Value>,
    pub splits: HashMap<String, Vec<String>>, // from arraydim value to uint values
}
