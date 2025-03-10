use crate::ast::Schedule;

#[derive(Clone, Debug)]
pub enum Node {
    Leaf {
        index: String,
    },
    Interior {
        index: String,
        op: char,
        children: Vec<(Node, String)>, // the child node and the index according to current `Node`
        schedule: Schedule,
    },
}

impl Node {
    pub fn get_leaves_mut(&mut self) -> Vec<&mut Node> {
        match self {
            Node::Leaf { .. } => vec![self],
            Node::Interior { children, .. } => children
                .iter_mut()
                .flat_map(|(child, _index)| child.get_leaves_mut())
                .collect(),
        }
    }

    pub fn index(&self) -> String {
        match self {
            Self::Leaf { index, .. } | Self::Interior { index, .. } => index.to_string(),
        }
    }

    pub fn children(&self) -> Option<&Vec<(Node, String)>> {
        match self {
            Node::Leaf { .. } => None,
            Node::Interior { children, .. } => Some(&children),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Graph {
    nodes: Vec<Node>,
}

impl Graph {
    pub fn new(nodes: Vec<Node>) -> Self {
        Self { nodes }
    }

    pub fn root(&self) -> &Node {
        &self.nodes[0]
    }
}
