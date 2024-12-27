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

use std::{fs, process::Command, env, path::Path};

// cargo fmt
fn format_rust_code(code: String) -> String {
    let path = "/tmp/tmp.rs";
    fs::write(&path, code).unwrap();
    Command::new("rustfmt").arg(&path).status().unwrap();
    fs::read_to_string(&path).unwrap()
}
// cargo fmt

fn main() -> Result<(), String> {
    // Get input and output file paths from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err("Usage: program <input_file> <output_file>".to_string());
    }
    let input_file = &args[1];
    let output_file = &args[2];

    // Read the input file
    let input = fs::read_to_string(input_file)
        .map_err(|e| format!("Failed to read input file: {}", e))?;

    let (ast, expr_bank) = Parser::new(&input)?.parse().unwrap();
    let backend = RustBackend {};
    let renderer: renderer::Renderer<RustBackend> = Renderer::new(backend, ast, expr_bank);
    let code = format!("fn main() {{ let f = {};}}", renderer.render().unwrap());

    // Format the code and write to the output file
    let formatted_code = format_rust_code(code);
    fs::write(output_file, formatted_code)
        .map_err(|e| format!("Failed to write output file: {}", e))?;

    Ok(())
}
