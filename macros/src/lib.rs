use compiler::parser::Parser;
use compiler::backend::Generator;
use compiler::backend::rust::RustBackend;

use proc_macro::TokenStream;

#[proc_macro]
pub fn i(input: TokenStream) -> TokenStream {
    let (ast, expr_bank) = Parser::new(&input.to_string()).unwrap().parse().unwrap();
    let backend = RustBackend {};
    let generator: Generator<RustBackend> = Generator::new(backend, ast, expr_bank);
    let code = generator.gen().unwrap();
    code.parse().unwrap()
}
