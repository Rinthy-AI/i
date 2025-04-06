mod ast;
mod backend;
mod block;
mod graph;
mod lowerer;
mod parser;
mod tokenizer;

use backend::cuda::CudaBackend;

use crate::backend::block::BlockBackend;
use crate::backend::rust::RustBackend;
use crate::backend::Render;
use crate::graph::Graph;
use crate::lowerer::Lowerer;
use crate::parser::Parser;

use std::io::Read;
use std::{env, fs, io, process::Command};

// Formats Rust code using rustfmt
fn format_rust_code(code: String) -> String {
    let path = "/tmp/tmp.rs";
    fs::write(&path, code).unwrap();
    Command::new("rustfmt").arg(&path).status().unwrap();
    fs::read_to_string(&path).unwrap()
}

fn i(input: &str) -> Graph {
    let (_ast, expr_bank) = Parser::new(&input).unwrap().parse().unwrap();
    let crate::ast::Expr::Index(_) = expr_bank.0[0] else {
        panic!("expression is not of variant Index")
    };
    Graph::from_expr_bank(&expr_bank)
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    // Parse command-line arguments
    let mut output_path: Option<String> = None;

    let m = i("ik*kj~ijk");
    let a = i("+ijk~ij");
    let graph = a.compose(&m);

    // lower
    let block = Lowerer::new().lower(&graph);

    let formatted_code = format_rust_code(format_rust_code(RustBackend::render(&block)));

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
