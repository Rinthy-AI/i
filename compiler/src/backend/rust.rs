use std::fs;
use std::io::Error;
use std::path::PathBuf;
use std::process::Command;

use crate::backend::{Backend, Build, Render};
use crate::block::{Arg, Block, Expr, Program, Statement, Type};

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
    fn render(program: &Program) -> String {
        format!(
            r#"
#[repr(C)]
pub struct Tensor<'a> {{
    pub data: *const f32,
    pub shape: *const usize,
    pub ndim: usize,
    pub _marker: std::marker::PhantomData<&'a [f32]>,
}}

#[repr(C)]
pub struct TensorMut<'a> {{
    pub data: *mut f32,
    pub shape: *const usize,
    pub ndim: usize,
    pub _marker: std::marker::PhantomData<&'a mut [f32]>,
}}

{}

{}
"#,
            Self::render_block(&program.library),
            Self::render_exec(&program.exec)
        )
    }
}

impl RustBackend {
    fn render_block(block: &Block) -> String {
        block
            .statements
            .iter()
            .map(|statement| Self::render_statement(&statement))
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn render_type(type_: &Type) -> String {
        match type_ {
            Type::Int(_) => "usize".to_string(),
            Type::Array(mutable) | Type::ArrayRef(mutable) => {
                format!("&{}[f32]", if *mutable { "mut " } else { "" })
            }
        }
    }
    fn render_op(expr: &Expr) -> String {
        let Expr::Op { op, inputs } = expr else {
            panic!("Expected `Op` variant of `Expr`")
        };
        match op {
            '>' => match inputs.len() {
                1 => format!(
                    "if {} > 0. {{ {} }} else {{ 0. }}",
                    Self::render_expr(&inputs[0]),
                    Self::render_expr(&inputs[0]),
                ),
                2 => format!(
                    "if {} > {} {{ {} }} else {{ {} }}",
                    Self::render_expr(&inputs[0]),
                    Self::render_expr(&inputs[1]),
                    Self::render_expr(&inputs[0]),
                    Self::render_expr(&inputs[1]),
                ),
                _ => panic!("Expected 1 or 2 inputs to op [>]."),
            },
            '^' => {
                assert!(inputs.len() == 1, "Expected 1 input to op [^].");
                format!("({} as f64).exp() as f32 ", Self::render_expr(&inputs[0]))
            }
            '$' => {
                assert!(inputs.len() == 1, "Expected 1 input to op [$].");
                format!("({} as f64).ln() as f32", Self::render_expr(&inputs[0]))
            }
            c => {
                if inputs.len() == 1 {
                    if *c == '-' {
                        return format!("-({})", Self::render_expr(&inputs[0]));
                    }
                    if *c == '/' {
                        return format!("1. / {}", Self::render_expr(&inputs[0]));
                    }
                }
                format!(
                    "({})",
                    inputs
                        .iter()
                        .map(|input| Self::render_expr(&input))
                        .collect::<Vec<_>>()
                        .join(&format!(" {op} "))
                )
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
            Expr::Op { .. } => Self::render_op(&expr),
            Expr::Indexed { ident, index } => format!("{ident}[{}]", Self::render_expr(&index),),
        }
    }

    fn render_exec(statement: &Statement) -> String {
        if let Statement::Function { ident, args, body } = &statement {
            format!(
                r#"
#[no_mangle]
pub unsafe extern "C"
fn f(inputs: *const Tensor, n_inputs: usize, output: *mut TensorMut) {{
    let inputs = std::slice::from_raw_parts(inputs, n_inputs);
    let output = &mut *output;

    {}
}}
"#,
                Self::render_block(&body),
            )
        } else {
            panic!("Found non-`Function` `Statement` for executive function.")
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
                    Self::render_block(body)
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
                Self::render_block(&body),
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
