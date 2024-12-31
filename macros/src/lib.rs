use compiler::backend::rust::RustBackend;
use compiler::parser::Parser;
use compiler::render::Render;

use proc_macro::TokenStream;

#[proc_macro]
pub fn i(input: TokenStream) -> TokenStream {
    let (ast, expr_bank) = Parser::new(&input.to_string()).unwrap().parse().unwrap();
    assert_eq!(expr_bank.0.len(), 1);

    // get IndexExpr
    let compiler::ast::Expr::Index(ref expr) = expr_bank.0[0]
    else { panic!("expression is not of variant Index") };

    // lower
    let block = compiler::lowerer::lower(&expr);

    let code = RustBackend::render(&block);
    code.parse().unwrap()
}
