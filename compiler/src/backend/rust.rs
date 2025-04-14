use std::fs;
use std::io::Error;
use std::path::PathBuf;
use std::process::Command;

use crate::backend::{Backend, Build, Render};
use crate::block::{Arg, Block, Expr, Statement, Type};

pub struct RustBackend;

impl Backend for RustBackend {}

impl Build for RustBackend {
    fn build(source: &str) -> Result<PathBuf, Error> {
        let path_base = "/tmp/ilang";
        let source_path = format!("{path_base}.rs");
        let dylib_path = format!("{path_base}.so");
        fs::write(&source_path, source)?;
        let build = Command::new("rustc")
            .args([
                "--crate-type=dylib",
                "-o",
                &dylib_path,
                &source_path,
                "-A",
                "warnings",
            ])
            .status();
        if let Err(e) = build {
            return Err(e);
        }
        let exit = build.unwrap();
        if !exit.success() {
            return Err(Error::last_os_error());
        }
        Ok(PathBuf::from(dylib_path))
    }
}

impl Render for RustBackend {
    fn render(block: &Block) -> String {
        block
            .statements
            .iter()
            .map(|statement| Self::render_statement(&statement))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl RustBackend {
    fn render_type(type_: &Type) -> String {
        match type_ {
            Type::Int(_) => "usize".to_string(),
            Type::Array(mutable) | Type::ArrayRef(mutable) => {
                format!("&{}[f32]", if *mutable { "mut " } else { "" })
            }
        }
    }
    fn render_expr(expr: &Expr) -> String {
        match expr {
            Expr::Alloc {
                initial_value,
                shape,
            } => {
                format!(
                    "&mut vec![{}; {}][..]",
                    format!("{:.1}", initial_value), // using `.to_string()` won't produce decimal
                    format!("{}", shape.join(" * ")),
                )
            }
            Expr::Ident(s) => s.to_string(),
            Expr::Ref(s, _mutable) => format!("{s}"),
            Expr::Int(x) => format!("{x}"),
            Expr::Op { op, inputs } => format!(
                "({})",
                inputs
                    .iter()
                    .map(|input| Self::render_expr(&input))
                    .collect::<Vec<_>>()
                    .join(&format!(" {op} "))
            ),
            Expr::Indexed { ident, index } => format!("{ident}[{}]", Self::render_expr(&index),),
        }
    }

    fn render_statement(statement: &Statement) -> String {
        match statement {
            Statement::Assignment { left, right } => format!(
                "{} = {};",
                Self::render_expr(left),
                Self::render_expr(right)
            ),
            Statement::Declaration {
                ident,
                value,
                type_,
            } => {
                let (Type::Int(mutable) | Type::Array(mutable) | Type::ArrayRef(mutable)) = type_;
                format!(
                    "let {}{ident}: {} = {};",
                    if *mutable { "mut " } else { "" },
                    Self::render_type(type_),
                    Self::render_expr(value)
                )
            }
            Statement::Skip { index, bound } => format!("if {index} >= {bound} {{ continue; }}"),
            Statement::Loop {
                index, bound, body, ..
            } => {
                format!(
                    "for {index} in 0..{} {{ {} }}",
                    Self::render_expr(&bound),
                    Self::render(body)
                )
            }

            Statement::Function { ident, args, body } => format!(
                "#[no_mangle]\nfn {ident}({}) {{{}}}",
                args.iter()
                    .map(|Arg { type_, ident }| {
                        let (Type::Int(_) | Type::Array(_) | Type::ArrayRef(_)) = type_;
                        format!("{}: {}", Self::render_expr(ident), Self::render_type(type_),)
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
                Self::render(&body),
            ),
            Statement::Return { value } => Self::render_expr(&value),
            Statement::Call { ident, args } => format!(
                "{ident}({});",
                args.iter()
                    .map(|Arg { ident, .. }| Self::render_expr(&ident))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        }
    }
}
