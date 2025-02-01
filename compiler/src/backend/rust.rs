use std::ops::{Index, IndexMut};

use crate::block::{Arg, Block, Expr, Statement, Type};
use crate::render::Render;

pub struct RustBackend;
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
            Type::Array(_) => "Vec<f32>".to_string(),
            Type::ArrayRef(mutable) => format!("&{}Vec<f32>", if *mutable { "mut " } else { "" }),
        }
    }
    fn render_expr(expr: &Expr) -> String {
        match expr {
            Expr::Alloc {
                initial_value,
                shape,
            } => {
                format!(
                    "vec![{}; {}]",
                    format!("{:.1}", initial_value), // using `.to_string()` won't produce decimal
                    format!("{}", shape.join(" * ")),
                )
            }
            Expr::Ident(s) => s.to_string(),
            Expr::Ref(s, mutable) => format!("&{}{s}", if *mutable { "mut " } else { "" }),
            Expr::Int(x) => format!("{x}"),
            Expr::Op { op, inputs } => {
                let prec = |c: char| match c {
                    '*' | '/' => 2,
                    '+' | '-' => 1,
                    _ => 0,
                };
                let mp = prec(*op);
                let parts: Vec<String> = inputs
                    .iter()
                    .map(|child| {
                        let s = Self::render_expr(child);
                        match child {
                            Expr::Op { op: cop, .. } => {
                                let cp = prec(*cop);
                                if cp < mp || cp == 0 {
                                    format!("({s})")
                                } else {
                                    s
                                }
                            }
                            _ => s,
                        }
                    })
                    .collect();
                if mp == 0 {
                    format!("({})", parts.join(&format!(" {} ", op)))
                } else {
                    parts.join(&format!(" {} ", op))
                }
            }
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
                "fn {ident}({}) {{{}{}}}",
                //"|{}| {{{}}}",
                args.iter()
                    .map(|Arg { type_, ident }| {
                        let (Type::Int(mutable) | Type::Array(mutable) | Type::ArrayRef(mutable)) =
                            type_;
                        format!(
                            "{}{}: {}",
                            if *mutable { "mut " } else { "" },
                            Self::render_expr(ident),
                            Self::render_type(type_),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
                Self::get_copies(&args),
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

    fn get_copies(args: &Vec<Arg>) -> String {
        struct ArgGroup {
            array_ident: String,
            dim_idents: Vec<String>,
        }

        let mut args = args.iter().peekable();
        let mut arg_groups: Vec<ArgGroup> = vec![];

        while let Some(Arg {
            type_: Type::ArrayRef(_),
            ident: Expr::Ident(ident),
        }) = args.peek()
        {
            let mut arg_group = ArgGroup {
                array_ident: ident.clone(),
                dim_idents: vec![],
            };
            args.next();
            while let Some(Arg {
                type_: Type::Int(_),
                ident: Expr::Ident(ident),
            }) = args.peek()
            {
                arg_group.dim_idents.push(ident.clone());
                args.next();
            }
            arg_groups.push(arg_group);
        }

        arg_groups
            .iter()
            .map(
                |ArgGroup {
                     array_ident,
                     dim_idents,
                 }| {
                    let size = std::iter::once("4".to_string())
                        .chain(dim_idents.iter().map(|s| s.clone()))
                        .collect::<Vec<_>>()
                        .join(" * ");

                    format!(
                        r#"
                    /*
                    float *h_{array_ident} = {array_ident};
                    size_t size = {size} * sizeof(float);
                    cudaMalloc(&{array_ident}, {size});
                    cudaMemcpy({array_ident}, h_{array_ident}, {size}, cudaMemcpyHostToDevice);
                    */
                "#
                    )
                },
            )
            .collect::<Vec<_>>()
            .join("\n")
    }
}
