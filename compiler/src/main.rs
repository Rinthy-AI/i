mod ast;
mod backend;
mod block;
mod lowerer;
mod generator;
mod parser;
mod tokenizer;
use crate::backend::rust::RustBackend;
use crate::generator::Generator;
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
    let generator: generator::Generator<RustBackend> = Generator::new(backend, ast, expr_bank);
    let code = format!("let f = {};", generator.gen().unwrap());
    //println!("{}", format_rust_code(code));
    println!("{}", code);

    Ok(())
}
