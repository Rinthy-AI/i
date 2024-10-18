use std::ops::{Index, IndexMut};

use crate::backend::Backend;

pub struct RustBackend;
impl Backend for RustBackend {
    fn gen_kernel(&self, id: String, args: Vec<String>, return_: String, body: String) -> String {
        let arg_list = args
            .iter()
            .map(|arg| format!("{}", arg))
            .collect::<Vec<_>>()
            .join(", ");
        format!("|{arg_list}| -> {return_} {{ {body} }}")
    }
    fn get_arg_declaration_string(&self, id: String) -> String {
        format!("{}: Array", id)
    }
    fn get_return_type_string(&self) -> String {
        "Array".to_string()
    }
    fn get_var_declaration_string(&self, id: String, value: String) -> String {
        format!("let mut {id} = {value};")
    }
    fn dim_size_string(id: String, dim: usize) -> String {
        format!("{id}.shape[{dim}]")
    }
    fn get_out_array_declaration_string(
        out_dim_string: String,
        op_identity_string: String,
    ) -> String {
        format!("let mut out = Array::new(vec![{out_dim_string}], {op_identity_string});")
    }
    fn get_indexed_array_string(&self, id: String, index_vec: &Vec<String>) -> String {
        format!("{id}[&[{}]]", index_vec.join(", "))
    }
    fn make_loop_string(&self, c: char, body: String) -> String {
        format!("for {c} in 0..{} {{ {body} }}", format!("n{c}"))
    }
    fn get_return_string(&self, id: String) -> String {
        id
    }
    fn get_assert_eq_string(&self, left: String, right: String) -> String {
        format!("assert_eq!({left}, {right});")
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
        Array {
            data: vec![initial_value; size],
            shape,
        }
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
