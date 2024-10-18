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
