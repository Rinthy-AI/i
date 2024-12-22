mod ast;
mod backend;
mod block;
mod lowerer;
mod parser;
mod renderer;
mod tokenizer;
use crate::backend::rust::RustBackend;
use crate::renderer::Renderer;
use crate::parser::Parser;

// cargo fmt
use std::{fs, process::Command};
fn format_rust_code(code: String) -> String {
    let path = "/tmp/tmp.rs";
    fs::write(&path, code).unwrap();
    Command::new("rustfmt").arg(&path).status().unwrap();
    fs::read_to_string(&path).unwrap()
}
// cargo fmt

fn main() -> Result<(), String> {
    let input = r#"
        m: ik*kj~ijk
        a: +ijk~ij
        m.a
    "#;

    /*

    ik*kj~ijk

    +ijk~ij |

    */

    //println!("{:#?}", Parser::new(input)?.parse());

    let (ast, expr_bank) = Parser::new(input)?.parse().unwrap();
    let backend = RustBackend {};
    let renderer: renderer::Renderer<RustBackend> = Renderer::new(backend, ast, expr_bank);
    let code = format!("let f = {};", renderer.render().unwrap());
    //println!("{}", format_rust_code(code));
    println!("{}", code);

    Ok(())
}
