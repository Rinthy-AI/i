use crate::ast::{ ScalarOp, Schedule };

#[derive(Clone, Debug)]
pub enum Node {
    Leaf{
        index: String
    },
    Interior {
        index: String,
        op: ScalarOp,
        children: Vec<Node>,
        schedule: Schedule,
    }
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
}


#[derive(Clone, Debug)]
pub struct Graph {
    pub root: Node,
}

impl Graph {
    //pub fn get_leaves(&self) -> Vec<&Node> {
    //    self.root.get_leaves()
    //}
}

