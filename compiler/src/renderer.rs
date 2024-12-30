use crate::ast::{
    BinaryOp, Combinator, Expr, ExprBank, IndexExpr, NoOp, ScalarOp, Symbol, UnaryOp, AST,
};
use crate::backend::Backend;
use crate::block::{Block, Expr as BlockExpr, Statement};
use crate::lowerer::lower;

pub struct Renderer<B> {
    backend: B,
    ast: AST,
    expr_bank: ExprBank,
}

impl<B: Backend> Renderer<B> {
    pub fn new(backend: B, ast: AST, expr_bank: ExprBank) -> Self {
        Self {
            backend,
            ast,
            expr_bank,
        }
    }

    pub fn render(&self) -> Result<String, String> {
        Ok(self.render_expr_bank()?)
    }

    pub fn render_expr_bank(&self) -> Result<String, String> {
        // index of the anonymous index. will be outside iteration if it does not exist.
        let anon_ind = self.expr_bank.0.len() - 1;
        let module = self
            .expr_bank
            .0
            .iter()
            .enumerate()
            .map(|(ind, expr)| {
                self.render_expr(expr, if ind == anon_ind { None } else { Some(ind) })
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("\n\n");

        Ok(self.backend.gen_scope(module))
    }

    pub fn render_expr(&self, expr: &Expr, ind: Option<usize>) -> Result<String, String> {
        let id = match ind {
            Some(ind) => Some(format!("f{}", ind)),
            None => None,
        };
        let args = self.get_args(expr, 0);
        let arg_declaration_strings = args
            .iter()
            .map(|arg| self.backend.get_arg_declaration_string(arg.to_string()))
            .collect::<Vec<String>>();
        let return_ = self.backend.get_return_type_string();
        let body = match expr {
            Expr::Index(index_expr) => self.render_index_expr_body(&index_expr),
            Expr::Combinator(combinator) => self.render_combinator_body(combinator, &args),
        };
        Ok(self
            .backend
            .gen_block(id, arg_declaration_strings, return_, body))
    }

    fn render_combinator_body(&self, combinator: &Combinator, args: &Vec<String>) -> String {
        match combinator {
            Combinator::Chain(first, second) => {
                // get_args is called twice on this expr, but oh well, not the end of the world
                let n_args_first = self.get_args(&self.expr_bank.0[first.0], 0).len();
                let (first_args, second_args_) = args.split_at(n_args_first);

                let first_id = format!("f{}", first.0);
                let first_call =
                    format!("{}", self.backend.gen_call(first_id, &first_args.to_vec()));

                let mut second_args = vec![first_call];
                second_args.extend_from_slice(second_args_);

                let second_id = format!("f{}", second.0);
                let second_call = self.backend.gen_call(second_id, &second_args);

                format!("{second_call}")
            }
        }
    }

    fn get_args(&self, expr: &Expr, arg_ct: usize) -> Vec<String> {
        match expr {
            Expr::Index(index_expr) => match index_expr {
                IndexExpr {
                    op: ScalarOp::BinaryOp(_),
                    out: _,
                    schedule: _,
                } => {
                    vec![format!("in{arg_ct}"), format!("in{}", arg_ct + 1)]
                }
                _ => vec![format!("in{arg_ct}")],
            },
            Expr::Combinator(Combinator::Chain(first, second)) => {
                let mut args = self.get_args(&self.expr_bank.0[first.0], 0);
                let second_args = self.get_args(&self.expr_bank.0[second.0], args.len());
                args.extend(second_args[1..].to_vec());
                args
            }
        }
    }

    fn render_index_expr_body(&self, index_expr: &IndexExpr) -> String {
        let n = lower(index_expr);

        let value_declaration_strings = &n.statements[1..]
            .iter()
            .map(|statement| B::render_statement(&statement))
            .collect::<Vec<_>>()
            .join("\n");

        let allocation_statement = &n.statements[0];
        let out_array_declaration_string = B::render_statement(&allocation_statement);

        let loop_string = B::render_statement(&n.loops[0]);

        let return_string = self.backend.get_return_string("out".to_string());

        format!(
            "
            // compute dims
            {value_declaration_strings}

            // initialize output Array
            {out_array_declaration_string}

            // loops
            {loop_string}

            // return
            {return_string}
        "
        )
    }
}
