use std::{io::Error, path::PathBuf};

use crate::block::Block;

pub mod block;
pub mod cuda;
pub mod rust;

pub trait Render {
    fn render(block: &Block) -> String;
}

#[allow(dead_code)]
// Not dead code, but the compiler thinks so...?
pub trait Build {
    fn build(source: &str) -> Result<PathBuf, Error>;
}

#[allow(dead_code)]
// Not dead code, but the compiler thinks so...?
pub trait Backend: Render + Build {}
