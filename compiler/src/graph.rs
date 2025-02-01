use crate::ast::Schedule;

#[derive(Clone, Debug)]
pub enum Node {
    Leaf {
        index: String,
    },
    Interior {
        index: String,
        op: char,
        children: Vec<Node>,
        schedule: Schedule,
    },
}

impl Node {
    pub fn get_leaves_mut(&mut self) -> Vec<&mut Node> {
        match self {
            Node::Leaf { .. } => vec![self],
            Node::Interior { children, .. } => children
                .iter_mut()
                .flat_map(|child| child.get_leaves_mut())
                .collect(),
        }
    }

    pub fn index(&self) -> String {
        match self {
            Self::Leaf { index, .. } | Self::Interior { index, .. } => index.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Graph {
    pub root: Node,
}
