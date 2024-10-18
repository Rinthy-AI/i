use compiler::parser::Parser;
use compiler::backend::Generator;
use compiler::rust_backend::RustBackend;

use proc_macro::TokenStream;

#[proc_macro]
pub fn i(input: TokenStream) -> TokenStream {

    let (ast, expr_bank) = Parser::new(&input.to_string()).unwrap().parse().unwrap();
    let backend = RustBackend {};
    let generator: Generator<RustBackend> = Generator::new(backend);
    let code = generator.gen(ast, expr_bank).unwrap();
    code.parse().unwrap()
}
