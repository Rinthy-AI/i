use std::{io::Error, path::PathBuf};

use crate::block::Program;

pub mod block;
pub mod cuda;
pub mod rust;

pub trait Render {
    fn render(program: &Program) -> String;
}

#[allow(dead_code)]
// Not dead code, but the compiler thinks so...?
pub trait Build {
    fn build(source: &str) -> Result<PathBuf, Error>;
}

#[allow(dead_code)]
// Not dead code, but the compiler thinks so...?
pub trait Backend: Render + Build {}
