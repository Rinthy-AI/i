mod backend;
mod ir;
mod parser;
mod tokenizer;
use crate::backend::Generator;
use crate::backend::rust::RustBackend;
use crate::parser::Parser;

// cargo fmt
use std::{fs, process::Command};
fn format_rust_code(code: String) -> String {
    let path = "/tmp/tmp.rs";
    fs::write(&path, code).unwrap();
    Command::new("rustfmt").arg(&path).status().unwrap();
    fs::read_to_string(&path).unwrap()
}
// cargo fmt

fn main() -> Result<(), String> {
    let input = r#"
        m: ik*kj~ijk
        a: +ijk~ij
        m.a
    "#;

    //println!("{:#?}", Parser::new(input)?.parse());

    let (ast, expr_bank) = Parser::new(input)?.parse().unwrap();
    let backend = RustBackend {};
    let generator: backend::Generator<RustBackend> = Generator::new(backend, ast, expr_bank);
    let code = format!("let f = {};", generator.gen().unwrap());
    //println!("{}", format_rust_code(code));
    println!("{}", code);

    Ok(())
}

use std::ops::{Index, IndexMut};

#[derive(Debug)]
struct Array {
    data: Vec<f32>,
    shape: Vec<usize>, // Dimensions of the array
}

impl Array {
    fn new(shape: Vec<usize>, initial_value: f32) -> Self {
        let size = shape.iter().product();
        Array {
            data: vec![initial_value; size],
            shape,
        }
    }

    // Define your affine transform to compute 1-D index from N-D indices
    fn affine_transform(&self, nd_indices: &[usize]) -> Option<usize> {
        if nd_indices.len() != self.shape.len() {
            return None;
        }

        // Example affine transform: here you can implement your actual logic
        let mut idx = 0;
        for (i, &dim_index) in nd_indices.iter().enumerate() {
            if dim_index >= self.shape[i] {
                return None;
            }
            idx = idx * self.shape[i] + dim_index; // Simple affine transformation
        }

        Some(idx)
    }
}

// Implement immutable indexing (using `[]`)
impl Index<&[usize]> for Array {
    type Output = f32;

    fn index(&self, indices: &[usize]) -> &Self::Output {
        let idx = self.affine_transform(indices).expect("Invalid index");
        &self.data[idx]
    }
}

// Implement mutable indexing (using `[]`)
impl IndexMut<&[usize]> for Array {
    fn index_mut(&mut self, indices: &[usize]) -> &mut Self::Output {
        let idx = self.affine_transform(indices).expect("Invalid index");
        &mut self.data[idx]
    }
}

//---- PASTE YOUR GENERATED FUNCTION(S) HERE ------
//-------------------------------------------------

//fn main() {
//    let mut a = Array::new(vec![2, 2], 0.);
//    let mut b = Array::new(vec![2, 2], 0.);
//
//    // 90-degree counterclockwise rotation matrix
//    a[&[0, 0]] =  0.;
//    a[&[0, 1]] = -1.;
//    a[&[1, 0]] =  1.;
//    a[&[1, 1]] =  0.;
//
//    // some other matrix
//    b[&[0, 0]] =  1.;
//    b[&[0, 1]] =  2.;
//    b[&[1, 0]] =  3.;
//    b[&[1, 1]] =  4.;
//
//    // matmul
//    // |0 -1||1 2| = |0 0| + |-3 -4| = |-3 -4|
//    // |1  0||3 4|   |1 2|   | 0  0|   | 1  2|
//    let c = f_a(f_p(a, b));
//
//    println!("{:#?}", c);
//}
