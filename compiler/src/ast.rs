use std::collections::HashMap;

#[derive(Debug)]
pub struct AST(pub Vec<NamedExpr>, pub ExprRef);

#[derive(Debug)]
pub struct NamedExpr {
    pub ident: Symbol,
    pub expr_ref: ExprRef,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Index(IndexExpr),
    Combinator(Combinator),
}

/// Holds all Exprs
#[derive(Debug)]
pub struct ExprBank(pub Vec<Expr>);

/// An index into the ExprBank
#[derive(Clone, Copy, Debug)]
pub struct ExprRef(pub usize);

#[derive(Clone, Debug)]
pub struct IndexExpr {
    pub op: ScalarOp,
    pub out: Symbol,
    pub schedule: Schedule,
}

#[derive(Clone, Debug)]
pub struct Schedule {
    // Should we have a `SplitTable` AST type? What about `Int` and using it and `Symbol` here?
    pub splits: HashMap<char, Vec<usize>>, // loop index, split factors
    pub loop_order: Vec<(char, usize)>, // loop index, position in split list +1 (0 reserved for base loop)
}

#[derive(Clone, Debug)]
pub enum ScalarOp {
    BinaryOp(BinaryOp),
    UnaryOp(UnaryOp),
    NoOp(NoOp),
}

#[derive(Clone, Debug)]
pub enum BinaryOp {
    Mul(Symbol, Symbol),
    Add(Symbol, Symbol),
}

#[derive(Clone, Debug)]
pub enum UnaryOp {
    Prod(Symbol),
    Accum(Symbol),
}

#[derive(Clone, Debug)]
pub struct NoOp(pub Symbol);

#[derive(Clone, Debug)]
pub enum Combinator {
    Chain(ExprRef, ExprRef),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Symbol(pub String);
