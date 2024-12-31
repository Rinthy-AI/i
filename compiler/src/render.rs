use crate::block::Block;

pub trait Render {
    fn render(block: &Block) -> String;
}
