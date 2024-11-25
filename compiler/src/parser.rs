use std::collections::HashMap;
use std::fmt;

use crate::ir::{
    BinaryOp, Combinator, Dependency, Expr, ExprBank, ExprRef, Library, NamedExpr, NoOp, Program,
    ScalarOp, Symbol, UnaryOp, AST,
};
use crate::tokenizer::{Token, Tokenizer};

#[derive(Debug)]
pub enum ParseError {
    InvalidToken { expected: String },
    UnrecognizedSymbol { symbol: Symbol },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidToken { expected } => {
                write!(f, "Invalid token: Expected {expected}.")
            }
            ParseError::UnrecognizedSymbol { symbol } => {
                write!(f, "Unrecognized Symbol: {}.", symbol.0)
            }
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug)]
pub struct SymbolTable(HashMap<Symbol, ExprRef>);

impl SymbolTable {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn add(&mut self, symbol: Symbol, expr: ExprRef) {
        self.0.insert(symbol, expr);
    }

    fn get(&self, symbol: &Symbol) -> Option<ExprRef> {
        self.0.get(symbol).cloned()
    }
}

pub struct Parser<'a> {
    tokenizer: Tokenizer<'a>,
    pub symbol_table: SymbolTable, // TODO: remove pub
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Result<Self, String> {
        Ok(Self {
            tokenizer: Tokenizer::new(input)?,
            symbol_table: SymbolTable::new(),
        })
    }

    pub fn parse(&mut self) -> Result<(AST, ExprBank), ParseError> {
        let mut named_exprs = vec![];
        let mut expr_bank = ExprBank(Vec::new());
        while let Token::Colon = self.tokenizer.peek()[1] {
            let named_expr = self.parse_named_expr(&mut expr_bank)?;
            named_exprs.push(named_expr);
        }
        match self.tokenizer.peek() {
            [Token::EOF, _] => Ok((AST::Library(Library(named_exprs)), expr_bank)),
            _ => {
                let expr = self.parse_expr()?;
                expr_bank.0.push(expr);
                Ok((
                    AST::Program(Program(named_exprs, ExprRef(expr_bank.0.len() - 1))),
                    expr_bank,
                ))
            }
        }
    }

    fn parse_named_expr(&mut self, expr_bank: &mut ExprBank) -> Result<NamedExpr, ParseError> {
        let ident = self.parse_symbol()?;
        match self.tokenizer.next() {
            Token::Colon => {
                let expr = self.parse_expr()?;
                expr_bank.0.push(expr);
                let expr_ref = ExprRef(expr_bank.0.len() - 1);
                self.symbol_table.add(ident.clone(), expr_ref);
                Ok(NamedExpr(ident, expr_ref))
            }
            _ => Err(ParseError::InvalidToken {
                expected: "Colon".to_string(),
            }),
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        match self.tokenizer.peek() {
            [_, Token::Dot] => Ok(Expr::Combinator(self.parse_combinator()?)),
            [Token::Operator(_), _] | [_, Token::Operator(_)] | [_, Token::Squiggle] => {
                Ok(Expr::Dependency(self.parse_dependency()?))
            }
            _ => Err(ParseError::InvalidToken {
                expected: "Dependency or Dot".to_string(),
            }),
        }
    }

    fn parse_dependency(&mut self) -> Result<Dependency, ParseError> {
        let scalarop = self.parse_scalarop()?;
        match self.tokenizer.next() {
            Token::Squiggle => Ok(Dependency(scalarop, self.parse_symbol()?)),
            _ => Err(ParseError::InvalidToken {
                expected: "Squiggle".to_string(),
            }),
        }
    }

    fn parse_scalarop(&mut self) -> Result<ScalarOp, ParseError> {
        match self.tokenizer.peek() {
            [Token::Operator(_), _] => Ok(ScalarOp::UnaryOp(self.parse_unaryop()?)),
            [Token::Symbol(_), Token::Operator(_)] => {
                Ok(ScalarOp::BinaryOp(self.parse_binaryop()?))
            }
            [Token::Symbol(_), Token::Squiggle] => Ok(ScalarOp::NoOp(self.parse_noop()?)),
            _ => Err(ParseError::InvalidToken {
                expected: "[Operator]<Any>, [Symbol][Operator], [Symbol]<Any>".to_string(),
            }),
        }
    }

    fn parse_binaryop(&mut self) -> Result<BinaryOp, ParseError> {
        let left = self.parse_symbol()?;
        match self.tokenizer.next() {
            Token::Operator('*') => Ok(BinaryOp::Mul(left, self.parse_symbol()?)),
            Token::Operator('+') => Ok(BinaryOp::Add(left, self.parse_symbol()?)),
            _ => Err(ParseError::InvalidToken {
                expected: "Operator".to_string(),
            }),
        }
    }

    fn parse_unaryop(&mut self) -> Result<UnaryOp, ParseError> {
        match self.tokenizer.next() {
            Token::Operator('*') => Ok(UnaryOp::Prod(self.parse_symbol()?)),
            Token::Operator('+') => Ok(UnaryOp::Accum(self.parse_symbol()?)),
            _ => Err(ParseError::InvalidToken {
                expected: "Operator".to_string(),
            }),
        }
    }

    fn parse_noop(&mut self) -> Result<NoOp, ParseError> {
        Ok(NoOp(self.parse_symbol()?))
    }

    fn parse_combinator(&mut self) -> Result<Combinator, ParseError> {
        let left = self.parse_symbol()?;
        let left_expr_ref =
            self.symbol_table
                .get(&left)
                .ok_or_else(|| ParseError::UnrecognizedSymbol {
                    symbol: left.clone(),
                })?;

        match self.tokenizer.next() {
            Token::Dot => {
                let right = self.parse_symbol()?;
                let right_expr_ref = self.symbol_table.get(&right).ok_or_else(|| {
                    ParseError::UnrecognizedSymbol {
                        symbol: right.clone(),
                    }
                })?;
                Ok(Combinator::Chain(left_expr_ref, right_expr_ref))
            }
            _ => Err(ParseError::InvalidToken {
                expected: "Combinator".to_string(),
            }),
        }
    }

    fn parse_symbol(&mut self) -> Result<Symbol, ParseError> {
        match self.tokenizer.next() {
            Token::Symbol(s) => Ok(Symbol(s)),
            _ => Err(ParseError::InvalidToken {
                expected: "Symbol".to_string(),
            }),
        }
    }
}
