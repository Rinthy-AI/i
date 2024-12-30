use std::ops::{Index, IndexMut};

use crate::render::Render;
use crate::block::{ Block, Expr, Statement };

pub struct RustBackend;
impl Render for RustBackend {
    fn gen_scope(&self, body: String) -> String {
        format!("{{{body}}}")
    }
    fn get_arg_declaration_string(&self, id: String) -> String {
        format!("{}: Array", id)
    }
    fn get_return_type_string(&self) -> String {
        "Array".to_string()
    }
    fn gen_block(
        &self,
        id: Option<String>,
        args: Vec<String>,
        return_: String,
        body: String,
    ) -> String {
        let arg_list = args.join(", ");
        let anon = format!("|{arg_list}| -> {return_} {{ {body} }}");
        match id {
            Some(id) => format!("let {id} = {anon};"),
            None => format!("move {anon}"),
        }
    }
    fn gen_call(&self, id: String, arg_list: &Vec<String>) -> String {
        format!("{id}({})", arg_list.join(", "))
    }
    fn render(block: &Block) -> String {
        block.statements
            .iter()
            .map(|statement| Self::render_statement(&statement))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl RustBackend {
    fn render_expr(expr: &Expr) -> String {
        match expr {
            Expr::Alloc { initial_value, shape } => {
                format!(
                    "Array::new(vec![{}], {})",
                    format!("{}", shape.join(", ")),
                    format!("{:.1}", initial_value), // using `.to_string()` won't produce decimal
                )
            }
            Expr::ArrayDim { ident, dim } => format!("{ident}.shape[{dim}]"),
            Expr::Str(s) | Expr::Ident(s) => s.to_string(),
            Expr::Int(x) => format!("{x}"),
            Expr::Op {
                op,
                inputs,
            } => match inputs.len() {
                1 => format!("{}", Self::render_expr(&inputs[0])),
                2 => format!(
                    "({} {} {})",
                    Self::render_expr(&inputs[0]),
                    op,
                    Self::render_expr(&inputs[1])
                ),
                _ => panic!(),
            },
            Expr::Indexed { ident, index } => format!("{ident}[&[{}]]", index.join(", ")),
        }
    }

    fn render_statement(statement: &Statement) -> String {
        match statement {
            Statement::Assignment { left, right } => format!(
                "{} = {};",
                Self::render_expr(left),
                Self::render_expr(right)
            ),
            Statement::Declaration{ ident, value } => format!(
                "let mut {ident} = {};",
                Self::render_expr(value)
            ),
            Statement::Skip{ index, bound } => format!("if {index} >= {bound} {{ continue; }}"),
            Statement::Loop{ index, bound, body } => {
                format!("for {index} in 0..{bound} {{ {} }}", Self::render(body))
            }
            Statement::Return { value } => Self::render_expr(&value),
        }
    }
}

#[derive(Debug)]
pub struct Array {
    pub data: Vec<f32>,
    pub shape: Vec<usize>,
}

impl Array {
    pub fn new(shape: Vec<usize>, initial_value: f32) -> Self {
        let size = shape.iter().product();
        Self::with_data(shape, vec![initial_value; size])
    }

    pub fn with_data(shape: Vec<usize>, data: Vec<f32>) -> Self {
        assert_eq!(data.len(), shape.iter().product());
        Array { data, shape }
    }

    /// affine transform to compute 1-D index from N-D indices
    fn affine_transform(&self, nd_indices: &[usize]) -> Option<usize> {
        if nd_indices.len() != self.shape.len() {
            return None;
        }

        let mut idx = 0;
        for (i, &dim_index) in nd_indices.iter().enumerate() {
            if dim_index >= self.shape[i] {
                return None;
            }
            idx = idx * self.shape[i] + dim_index;
        }

        Some(idx)
    }
}

impl Index<&[usize]> for Array {
    type Output = f32;

    fn index(&self, indices: &[usize]) -> &Self::Output {
        let idx = self.affine_transform(indices).expect("Invalid index");
        &self.data[idx]
    }
}

impl IndexMut<&[usize]> for Array {
    fn index_mut(&mut self, indices: &[usize]) -> &mut Self::Output {
        let idx = self.affine_transform(indices).expect("Invalid index");
        &mut self.data[idx]
    }
}
