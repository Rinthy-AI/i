pub mod rust;

use crate::block::{ Block, Expr, Statement };

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
    fn get_indexed_array_string(&self, id: String, index_vec: &Vec<String>) -> String;
    fn make_loop_string(&self, index: String, bound: String, body: String) -> String;
    fn get_return_string(&self, id: String) -> String;
    fn get_assert_eq_string(&self, left: String, right: String) -> String;
    fn gen_call(&self, id: String, arg_list: &Vec<String>) -> String;
    fn gen_scope(&self, body: String) -> String;
    fn gen_div_string(&self, numerator: String, divisor: String) -> String;

    fn render_expr(expr: &Expr) -> String;
    fn render_statement(statement: &Statement) -> String;
    fn render(block: &Block) -> String;
}
