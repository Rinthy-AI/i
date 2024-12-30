pub mod rust;

use crate::block::Block;

pub trait Backend {
    fn render(block: &Block) -> String;
    fn gen_scope(&self, body: String) -> String;
    fn get_arg_declaration_string(&self, id: String) -> String;
    fn get_return_type_string(&self) -> String;
    fn gen_block(
        &self,
        id: Option<String>,
        args: Vec<String>,
        return_: String,
        body: String,
    ) -> String;
    fn gen_call(&self, id: String, arg_list: &Vec<String>) -> String;
}
