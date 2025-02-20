use compiler::backend::rust::RustBackend;
use compiler::backend::Render;
use compiler::grapher;
use compiler::lowerer::Lowerer;
use compiler::parser::Parser;

use proc_macro::TokenStream;

#[proc_macro]
pub fn i(input: TokenStream) -> TokenStream {
    let (_ast, expr_bank) = Parser::new(&input.to_string()).unwrap().parse().unwrap();
    let graph = grapher::graph(&expr_bank);
    let block = Lowerer::new().lower(&graph);
    let code = RustBackend::render(&block);
    code.parse().unwrap()
}
