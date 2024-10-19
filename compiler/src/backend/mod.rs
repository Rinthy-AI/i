pub mod rust;

use std::collections::{HashMap, HashSet};

use crate::ir::{
    BinaryOp, Combinator, Dependency, Expr, ExprBank, ExprRef, Library, NamedExpr, NoOp, Program,
    ScalarOp, Symbol, UnaryOp, AST,
};

pub trait Backend {
    fn gen_kernel(&self, id: Option<String>, args: Vec<String>, return_: String, body: String) -> String;
    fn get_arg_declaration_string(&self, id: String) -> String;
    fn get_return_type_string(&self) -> String;
    fn get_var_declaration_string(&self, id: String, value: String) -> String;
    fn dim_size_string(id: String, dim: usize) -> String;
    fn get_out_array_declaration_string(
        out_dim_string: String,
        op_identity_string: String,
    ) -> String;
    fn get_indexed_array_string(&self, id: String, index_vec: &Vec<String>) -> String;
    fn make_loop_string(&self, c: char, body: String) -> String;
    fn get_return_string(&self, id: String) -> String;
    fn get_assert_eq_string(&self, left: String, right: String) -> String;
    fn gen_call(&self, id: String, arg_list: &Vec<String>) -> String;
    fn gen_scope(&self, body: String) -> String;
}

pub struct Generator<B> {
    backend: B,
    ast: AST,
    expr_bank: ExprBank
}

impl<B: Backend> Generator<B> {
    pub fn new(backend: B, ast: AST, expr_bank: ExprBank) -> Self {
        Self { backend, ast, expr_bank }
    }

    pub fn gen(&self) -> Result<String, String> {
        match self.ast {
            AST::Program(_) => Ok(self.gen_expr_bank(true)?),
            AST::Library(_) => Ok(self.gen_expr_bank(false)?),
        }
    }

    pub fn gen_expr_bank(&self, program: bool) -> Result<String, String> {
        // index of the anonymous index. will be outside iteration if it does not exist.
        let anon_ind = self.expr_bank.0.len() - (program as usize);
        let module = self.expr_bank
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
            Expr::Dependency(dependency) => self.gen_dependency_body(dependency, &args),
            Expr::Combinator(combinator) => self.gen_combinator_body(combinator, &args),
        };
        Ok(self
            .backend
            .gen_kernel(id, arg_declaration_strings, return_, body))
    }

    fn gen_combinator_body(&self, combinator: &Combinator, args: &Vec<String>) -> String {
        match combinator {
            Combinator::Chain(first, second) => {
                // get_args is called twice on this expr, but oh well, not the end of the world
                let n_args_first = self.get_args(&self.expr_bank.0[first.0], 0).len();
                let (first_args, second_args_) = args.split_at(n_args_first);

                let first_id = format!("f{}", first.0);
                let first_call = format!("{}", self.backend.gen_call(first_id, &first_args.to_vec()));

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
            Expr::Dependency(dependency) => match dependency {
                Dependency(ScalarOp::BinaryOp(_), _) => vec![
                    format!("in{arg_ct}"), format!("in{}", arg_ct + 1)
                ],
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

    fn gen_dependency_body(&self, dependency: &Dependency, args: &Vec<String>) -> String {
        let Dependency(scalar_op, result_index) = dependency;

        let out_dim_string = result_index
            .0
            .clone()
            .chars()
            .map(|c| format!("n{c}"))
            .collect::<Vec<_>>()
            .join(", ");

        let (input_index_strings, output_index_string) = dependency.get_index_strings();
        let index_strings = [
            input_index_strings.clone(),
            vec![output_index_string.clone()],
        ]
        .concat();

        let indices = index_strings
            .iter()
            .flat_map(|s| s.chars())
            .collect::<HashSet<char>>();

        // maps atomic index to vector over inputs, elements being flattened
        // sets of (input_index, dimension_index)
        let index_to_dims: HashMap<char, Vec<(usize, usize)>> = indices
            .iter()
            .map(|&c| {
                let flattened = input_index_strings
                    .iter()
                    .enumerate()
                    .flat_map(|(input_ind, input_index)| {
                        input_index
                            .chars()
                            .enumerate()
                            .filter(move |&(_, ch)| ch == c)
                            .map(move |(dim, _)| (input_ind, dim))
                    })
                    .collect::<Vec<_>>();
                (c, flattened)
            })
            .collect();

        let dim_strings = indices
            .iter()
            .map(|c| match index_to_dims[c].get(0) {
                Some(&(input_ind, dim)) => self.backend.get_var_declaration_string(
                    format!("n{c}"),
                    B::dim_size_string(format!("in{input_ind}"), dim),
                ),
                None => self
                    .backend
                    .get_var_declaration_string(format!("n{c}"), format!("1")),
            })
            .collect::<Vec<_>>()
            .join("\n    ");

        let out_array_declaration_string =
            B::get_out_array_declaration_string(out_dim_string, scalar_op.get_identity_string());

        let (input_index_vecs, output_index_vec, op_char) = dependency.get_index_vecs_and_op_char();

        let indexed_input_array_strings = input_index_vecs
            .iter()
            .enumerate()
            .map(|(ind, index_vec)| {
                self.backend
                    .get_indexed_array_string(format!("in{ind}"), &index_vec)
            })
            .collect::<Vec<_>>();
        let indexed_output_array_strings = self
            .backend
            .get_indexed_array_string("out".to_string(), &output_index_vec);

        let partial_op_string = match indexed_input_array_strings.len() {
            1 => {
                let x = &indexed_input_array_strings[0];
                format!("{x}")
            }
            2 => {
                let left = &indexed_input_array_strings[0];
                let right = &indexed_input_array_strings[1];
                format!("{left} {op_char} {right}")
            }
            _ => panic!(),
        };

        let op_string = format!(
            "{indexed_output_array_strings} = {indexed_output_array_strings} {op_char} ({partial_op_string});"
        );

        let mut loop_string = op_string;
        for (i, &c) in (0..indices.len()).rev().zip(indices.iter()) {
            loop_string = self.backend.make_loop_string(c, loop_string);
        }

        let return_string = self.backend.get_return_string("out".to_string());

        let dimension_assertions = index_to_dims
            .into_iter()
            .filter(|(_, v)| v.len() > 1)
            .map(|(c, v)| {
                let ((first_input_ind, first_dim), rest) = v.split_first().unwrap();
                rest.iter()
                    .map(|(x_input_ind, x_dim)| {
                        // (first, x)
                        let first_shape_str =
                            B::dim_size_string(format!("in{first_input_ind}"), *first_dim);
                        let x_shape_str = B::dim_size_string(format!("in{x_input_ind}"), *x_dim);
                        self.backend
                            .get_assert_eq_string(first_shape_str, x_shape_str)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<String>>()
            .join("\n    ");

        format!(
            "
            // compute dims
            {dim_strings}

            // assert dim constraints
            {dimension_assertions}

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

impl Dependency {
    // TODO: This pattern is really nasty. Maybe the `ScalarOp`-related enums should change
    /// Returns vector of Strings for all input indices and one String for output index
    fn get_index_strings(&self) -> (Vec<String>, String) {
        let Dependency(scalar_op, output_index) = self;
        let input_indices = match scalar_op {
            ScalarOp::BinaryOp(BinaryOp::Mul(i0, i1))
            | ScalarOp::BinaryOp(BinaryOp::Add(i0, i1)) => vec![i0.0.to_string(), i1.0.to_string()],
            ScalarOp::UnaryOp(UnaryOp::Prod(i0)) | ScalarOp::UnaryOp(UnaryOp::Accum(i0)) => {
                vec![i0.0.to_string()]
            }
            ScalarOp::NoOp(NoOp(i0)) => vec![i0.0.to_string()],
        };
        (input_indices, output_index.0.to_string())
    }

    /// Returns index vec for each input, index vec for output, op char
    fn get_index_vecs_and_op_char(&self) -> (Vec<Vec<String>>, Vec<String>, String) {
        let Dependency(scalar_op, output_index) = self;
        let (input_index_vec, op_char) = scalar_op.get_index_vecs_and_op_char();
        (input_index_vec, output_index.array_index_strings(), op_char)
    }
}

impl ScalarOp {
    fn get_identity_string(&self) -> String {
        match self {
            ScalarOp::BinaryOp(BinaryOp::Mul(_, _)) | ScalarOp::UnaryOp(UnaryOp::Prod(_)) => {
                "1.0".to_string()
            }
            ScalarOp::BinaryOp(BinaryOp::Add(_, _)) | ScalarOp::UnaryOp(UnaryOp::Accum(_)) => {
                "0.0".to_string()
            }
            ScalarOp::NoOp(NoOp(_)) => "0.0".to_string(),
        }
    }

    /// Returns index vec for each input and the op char
    fn get_index_vecs_and_op_char(&self) -> (Vec<Vec<String>>, String) {
        match self {
            ScalarOp::BinaryOp(BinaryOp::Mul(in0_index, in1_index)) => (
                vec![
                    in0_index.array_index_strings(),
                    in1_index.array_index_strings(),
                ],
                "*".to_string(),
            ),
            ScalarOp::BinaryOp(BinaryOp::Add(in0_index, in1_index)) => (
                vec![
                    in0_index.array_index_strings(),
                    in1_index.array_index_strings(),
                ],
                "+".to_string(),
            ),
            ScalarOp::UnaryOp(UnaryOp::Prod(in0_index)) => {
                (vec![in0_index.array_index_strings()], "*".to_string())
            }
            ScalarOp::UnaryOp(UnaryOp::Accum(in0_index)) => {
                (vec![in0_index.array_index_strings()], "+".to_string())
            }
            ScalarOp::NoOp(NoOp(in0_index)) => {
                (vec![in0_index.array_index_strings()], "+".to_string())
            }
        }
    }
}

impl Symbol {
    fn array_index_strings(&self) -> Vec<String> {
        self.0.chars().map(|c| c.to_string()).collect()
    }
}
