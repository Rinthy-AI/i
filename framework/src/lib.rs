use compiler;

pub fn i(input: &str) -> Result<Component, String> {
    let (_ast, expr_bank) = compiler::parser::Parser::new(&input.to_string())
        .unwrap()
        .parse()
        .unwrap();
    Ok(Component {
        graph: compiler::grapher::graph(&expr_bank),
        inputs: vec![],
    })
}

#[derive(Clone, Debug)]
pub struct Array {
    data: Vec<f32>,
    shape: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Component<'a> {
    pub graph: compiler::graph::Graph,
    inputs: Vec<Option<&'a Array>>,
}

impl<'a> Component<'a> {
    fn apply(&self, inputs: impl IntoIterator<Item = &'a Array>) -> Result<(), String> {
        let mut out = self.clone();
        let mut inputs = inputs.into_iter();

        for current in out.inputs.iter_mut() {
            if current.is_none() {
                if let Some(next_input) = inputs.next() {
                    *current = Some(next_input);
                }
            }
        }

        if inputs.next().is_some() {
            return Err("Too many inputs.".to_string());
        }

        Ok(())
    }

    pub fn chain(&self, other: &Component<'a>) -> Result<Component<'a>, String> {
        let mut out = other.clone();
        match out.graph.root.get_leaves_mut().first_mut() {
            Some(first) => {
                let input_dims = first.index().len();
                let output_dims = self.graph.root.index().len();
                assert_eq!(
                    input_dims, output_dims,
                    "Chaining {}-D Componenet into {}-D input.",
                    input_dims, output_dims,
                );
                **first = self.graph.root.clone();
                Ok(out)
            }
            _ => Err("Receiving Component in `Chain` takes no inputs.".to_string()),
        }
    }
}
