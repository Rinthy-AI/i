use compiler::backend::rust::RustBackend;
use compiler::parser::Parser;
use compiler::renderer::Renderer;

use proc_macro::TokenStream;

#[proc_macro]
pub fn i(input: TokenStream) -> TokenStream {
    let (ast, expr_bank) = Parser::new(&input.to_string()).unwrap().parse().unwrap();
    let backend = RustBackend {};
    let renderer: Renderer<RustBackend> = Renderer::new(backend, ast, expr_bank);
    let code = renderer.render().unwrap();
    code.parse().unwrap()
}
