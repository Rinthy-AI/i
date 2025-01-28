mod ast;
mod backend;
mod block;
mod graph;
mod grapher;
mod lowerer;
mod parser;
mod render;
mod tokenizer;

use crate::backend::rust::RustBackend;
use crate::lowerer::Lowerer;
use crate::parser::Parser;
use crate::render::Render;

use std::io::Read;
use std::{env, fs, io, process::Command};

// Formats Rust code using rustfmt
fn format_rust_code(code: String) -> String {
    let path = "/tmp/tmp.rs";
    fs::write(&path, code).unwrap();
    Command::new("rustfmt").arg(&path).status().unwrap();
    fs::read_to_string(&path).unwrap()
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    // Parse command-line arguments
    let mut input_path: Option<String> = None;
    let mut output_path: Option<String> = None;
    let mut target = "rust";

    let mut iter = args.iter().skip(1); // Skip the program name
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-t" | "--target" => {
                target = iter.next().ok_or("Error: Missing value for --target")?;
            }
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            other if input_path.is_none() => input_path = Some(other.to_string()),
            other if output_path.is_none() => output_path = Some(other.to_string()),
            _ => return Err("Error: Too many arguments".to_string()),
        }
    }

    // Validate the target platform
    if target != "rust" {
        return Err(format!("Error: Unsupported target '{}'", target));
    }

    // Read input
    let input = if let Some(path) = input_path {
        if path == "-" {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|e| format!("Failed to read from STDIN: {}", e))?;
            buffer
        } else {
            fs::read_to_string(path).map_err(|e| format!("Failed to read input file: {}", e))?
        }
    } else {
        return Err("Error: Missing input file".to_string());
    };

    // Process the input
    let (ast, expr_bank) = Parser::new(&input)?.parse().unwrap();
    let graph = grapher::graph(&expr_bank);

    // get IndexExpr
    let crate::ast::Expr::Index(ref expr) = expr_bank.0[0] else {
        panic!("expression is not of variant Index")
    };

    // lower
    let block = Lowerer::new().lower(&graph);
    println!("{:#?}", block);

    let code = RustBackend::render(&block);
    let formatted_code = format_rust_code(format!("fn main() {{ {code};}}"));
    //let formatted_code = code;

    //let formatted_code = format!("{:#?}", block);

    //let backend = RustBackend {};
    //let renderer: renderer::Renderer<RustBackend> = Renderer::new(backend, ast, expr_bank);
    //let code = format!("fn main() {{ let f = {};}}", renderer.render().unwrap());
    //let formatted_code = format_rust_code(code);

    // Write output
    if let Some(path) = output_path {
        if path == "-" {
            println!("{}", formatted_code);
        } else {
            fs::write(path, formatted_code)
                .map_err(|e| format!("Failed to write output file: {}", e))?;
        }
    } else {
        println!("{}", formatted_code);
    }

    Ok(())
}

// Prints the help message
fn print_help() {
    println!(
        r#"Usage: ic [OPTIONS] [INPUT] [OUTPUT]

Options:
  -t, --target <TARGET>  Specify the target platform (default: rust)
  -h, --help             Print this help message

Arguments:
  INPUT                  Path to the input file (use '-' for STDIN)
  OUTPUT                 Path to the output file (use '-' for STDOUT)"#
    );
}
