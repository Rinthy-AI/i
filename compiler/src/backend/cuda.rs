use crate::{
    block::{Arg, Block, Expr, Statement, Type},
    render::Render,
};

pub struct CudaBackend;

impl CudaBackend {
    fn render_type(type_: &Type) -> String {
        match type_ {
            Type::Int => "int".to_string(),
            Type::Array => "float*".to_string(),
        }
    }
    fn render_expr(expr: &Expr) -> String {
        match expr {
            Expr::Str(s) | Expr::Ident(s) => s.to_string(),
            Expr::Int(x) => format!("{x}"),
            Expr::Op { op, inputs } => match inputs.len() {
                1 => Self::render_expr(&inputs[0]).to_string(),
                2 => format!(
                    "({} {} {})",
                    Self::render_expr(&inputs[0]),
                    op,
                    Self::render_expr(&inputs[1])
                ),
                n => unreachable!("Op with {n} inputs"),
            },
            not_implemented => format!("TODO {:?}", not_implemented),
        }
    }
    fn render_statement(statement: &Statement) -> String {
        match statement {
            Statement::Assignment { left, right } => format!(
                "{} = {};",
                Self::render_expr(left),
                Self::render_expr(right)
            ),
            Statement::Declaration { ident, type_, .. } => {
                // TODO maybe fill with value... lowerer should handle initialization probably
                format!("{} {ident};", Self::render_type(type_))
            }
            Statement::Skip { index, bound } => format!("if ({index} >= {bound}) {{ continue; }}"),
            Statement::Loop { index, bound, body } => format!(
                "for (int {index} = 0; {index} < {bound}; {index}++) {{\n{}\n}}",
                Self::render(body)
            ),
            Statement::Return { value } => format!("return {};", Self::render_expr(value)),
            Statement::Function {
                ident,
                type_,
                args,
                body,
            } => {
                let kernel_arg_list = args
                    .iter()
                    .map(|Arg { type_, ident }| format!("{} d_{ident}", Self::render_type(type_)))
                    .collect::<Vec<_>>()
                    .join(",\n");
                let body_statements = Self::render(body);
                let lib_func_arg_list = args
                    .iter()
                    .map(|Arg { type_, ident }| {
                        format!(
                            r#"
    {} h_{ident},
    int {ident}_sz"#,
                            Self::render_type(type_)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",\n");
                let device_alloc = args
                    .iter()
                    .map(|Arg { type_, ident }| {
                        format!(
                            r#"{} d_{ident};
    cudaMalloc(&d_{ident}, {ident}_sz);
    cudaMemcpy(d_{ident}, h_{ident}, {ident}_sz, cudaMemcpyHostToDevice);
        "#,
                            Self::render_type(type_)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let kernel_launch_args = args
                    .iter()
                    .map(|Arg { ident, .. }| format!("d_{ident}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    r#"#include <cuda.h>

__global__ void d_{ident}(
    {kernel_arg_list}
) {{
    {body_statements}
}}

// TODO how do the device pointers get out of this function? probably output arrays
int {ident}({lib_func_arg_list}
) {{
    {device_alloc}
    const dim3 BLOCKS = (1,1,1);
    const dim3 THREADS = (1,1,1);
    d_{ident}<<<BLOCKS, THREADS>>>(
        {kernel_launch_args}
    );

    return 0;
}})"#,
                )
            }
        }
    }
}

impl Render for CudaBackend {
    fn render(block: &Block) -> String {
        block
            .statements
            .iter()
            .map(Self::render_statement)
            .collect::<Vec<_>>()
            .join("\n")
    }
}
