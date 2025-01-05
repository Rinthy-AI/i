use compiler::backend::rust::RustBackend;
use compiler::grapher;
use compiler::lowerer::Lowerer;
use compiler::parser::Parser;
use compiler::render::Render;

use proc_macro::TokenStream;

#[proc_macro]
pub fn i(input: TokenStream) -> TokenStream {
    let (ast, expr_bank) = Parser::new(&input.to_string()).unwrap().parse().unwrap();
    let graph = grapher::graph(&expr_bank);
    let block = Lowerer::new().lower(&graph);
    let code = RustBackend::render(&block);
    code.parse().unwrap()
}
