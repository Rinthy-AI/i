use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct Loop {
    pub bound: String, // ident of Value::ArrayDim, e.g., `ni`
    pub index_reconstruction: Option<String>, // ident of Value
}

#[derive(Clone, Debug)]
pub struct Alloc {
    pub initial_value: f32,
    pub shape: Vec<String>,
    pub index: Vec<String>,
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
pub struct Block {
    pub alloc: Alloc,
    pub accesses: Vec<Access>,
    pub loops: Vec<Loop>,
    pub op: char, // this can't be a char forever
    pub values: HashMap<String, Value>,
    pub splits: HashMap<String, Vec<String>>, // from arraydim value to uint values
}
