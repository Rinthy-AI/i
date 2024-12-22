pub mod rust;

use crate::ast::{
    BinaryOp, Combinator, Expr, ExprBank, IndexExpr, NoOp, ScalarOp, Symbol, UnaryOp, AST,
};
use crate::lowerer::lower;

use crate::block::{ArrayDim, Block, Value};

pub trait Backend {
    fn gen_block(
        &self,
        id: Option<String>,
        args: Vec<String>,
        return_: String,
        body: String,
    ) -> String;
    fn get_arg_declaration_string(&self, id: String) -> String;
    fn get_return_type_string(&self) -> String;
    fn get_var_declaration_string(&self, id: String, value: String) -> String;
    fn dim_size_string(id: String, dim: usize) -> String;
    fn get_out_array_declaration_string(
        out_dim_string: String,
        op_identity_string: String,
    ) -> String;
    fn get_indexed_array_string(&self, id: String, index_vec: &Vec<String>) -> String;
    fn make_loop_string(&self, index: String, bound: String, body: String) -> String;
    fn get_return_string(&self, id: String) -> String;
    fn get_assert_eq_string(&self, left: String, right: String) -> String;
    fn gen_call(&self, id: String, arg_list: &Vec<String>) -> String;
    fn gen_scope(&self, body: String) -> String;
    fn gen_div_string(&self, numerator: String, divisor: String) -> String;
}

pub struct Generator<B> {
    backend: B,
    ast: AST,
    expr_bank: ExprBank,
}

impl<B: Backend> Generator<B> {
    pub fn new(backend: B, ast: AST, expr_bank: ExprBank) -> Self {
        Self {
            backend,
            ast,
            expr_bank,
        }
    }

    pub fn gen(&self) -> Result<String, String> {
        Ok(self.gen_expr_bank()?)
    }

    pub fn gen_expr_bank(&self) -> Result<String, String> {
        // index of the anonymous index. will be outside iteration if it does not exist.
        let anon_ind = self.expr_bank.0.len() - 1;
        let module = self
            .expr_bank
            .0
            .iter()
            .enumerate()
            .map(|(ind, expr)| self.gen_expr(expr, if ind == anon_ind { None } else { Some(ind) }))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n\n");

        Ok(self.backend.gen_scope(module))
    }

    pub fn gen_expr(&self, expr: &Expr, ind: Option<usize>) -> Result<String, String> {
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
            Expr::Index(index_expr) => self.gen_index_expr_body(&index_expr),
            Expr::Combinator(combinator) => self.gen_combinator_body(combinator, &args),
        };
        Ok(self
            .backend
            .gen_block(id, arg_declaration_strings, return_, body))
    }

    fn gen_combinator_body(&self, combinator: &Combinator, args: &Vec<String>) -> String {
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

    fn gen_index_expr_body(&self, index_expr: &IndexExpr) -> String {
        let n = lower(index_expr);

        let value_declaration_strings = n
            .values
            .into_iter()
            .filter_map(|(ident, variable)| match variable {
                Value::ArrayDim(ArrayDim { input, dim }) => {
                    Some(self.backend.get_var_declaration_string(
                        ident,
                        B::dim_size_string(format!("in{input}"), dim),
                    ))
                }
                Value::Uint(u) => Some(
                    self.backend
                        .get_var_declaration_string(ident, u.to_string()),
                ),
                Value::Index(_) => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        let out_array_declaration_string = B::get_out_array_declaration_string(
            n.alloc.shape.join(", "),
            format!("{:.1}", n.alloc.initial_value), // .to_string() doesn't have decimal
        );

        let indexed_out_string = self
            .backend
            .get_indexed_array_string("out".to_string(), &n.alloc.index);

        let index_input_strings = n
            .accesses
            .iter()
            .enumerate()
            .map(|(ind, access)| {
                self.backend
                    .get_indexed_array_string(format!("in{ind}"), &access.indices)
            })
            .collect::<Vec<_>>();

        let partial_op_string = match index_input_strings.len() {
            1 => format!("{}", &index_input_strings[0]),
            2 => format!(
                "{} {} {}",
                &index_input_strings[0], n.op, &index_input_strings[1]
            ),
            _ => panic!(),
        };

        let op_string = format!(
            "{indexed_out_string} = {indexed_out_string} {} ({partial_op_string});",
            n.op
        );

        let mut loop_string = op_string;
        for l in n.loops.into_iter().rev() {
            let index = l.iterations[1..].to_string();
            let mut bound = l.iterations;
            if let Some(splits) = n.splits.get(&bound) {
                // TODO: This is also computed below
                let outer_tile_width_string = format!("({})", splits.join(" * "));
                bound =
                    format!("({bound} + {outer_tile_width_string} - 1)/{outer_tile_width_string}");
            }

            // reconstruct index and handle partial tiles inside split loops
            let index_reconstruction_string = match l.index_reconstruction {
                Some(base_index) => {
                    let splits = &n.splits[&format!("n{}", base_index)];

                    let n_splits = splits.len();
                    let outer_tile_width_string = (0..n_splits)
                        .map(|ind| format!("n{base_index}{ind}"))
                        .collect::<Vec<_>>()
                        .join(" * ");

                    let interim_loop_element_width_strings = (0..n_splits - 1)
                        .map(|ind| format!("n{base_index}{ind} * {base_index}{ind}"))
                        .collect::<Vec<_>>()
                        .join(" + ");

                    let index_reconstruction_string = format!(
                        "{base_index} * {outer_tile_width_string} + {interim_loop_element_width_strings} + {base_index}{}",
                        n_splits-1
                    );

                    format!(
                        "\n{}\n{}",
                        self.backend.get_var_declaration_string(
                            base_index.clone(),
                            index_reconstruction_string
                        ),
                        format!("if n{base_index} <= {base_index} {{ continue; }}")
                    )
                }
                None => "".to_string(),
            };

            loop_string = format!("{index_reconstruction_string}\n{loop_string}");
            loop_string = self.backend.make_loop_string(index, bound, loop_string);
        }

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
