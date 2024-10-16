mod ir;
mod parser;
mod tokenizer;
use crate::parser::Parser;

fn main() -> Result<(), String> {
    let input = r#"
        p: ik*kj~ijk
        a: +ijk~ij
        a.p
    "#;

    println!("{:#?}", Parser::new(input)?.parse());

    Ok(())
}
