use macros::i;
use compiler::backend::rust::Array;

fn main() {
    let p = i!(p: ik*kj~ijk);
    let a = i!(a: +ijk~ij);

    let mut x = Array::new(vec![2, 2], 0.);
    let mut y = Array::new(vec![2, 2], 0.);

    // 90-degree counterclockwise rotation matrix
    x[&[0, 0]] =  0.;
    x[&[0, 1]] = -1.;
    x[&[1, 0]] =  1.;
    x[&[1, 1]] =  0.;

    // some other matrix
    y[&[0, 0]] =  1.;
    y[&[0, 1]] =  2.;
    y[&[1, 0]] =  3.;
    y[&[1, 1]] =  4.;

    // matmul
    // |0 -1||1 2| = |0 0| + |-3 -4| = |-3 -4|
    // |1  0||3 4|   |1 2|   | 0  0|   | 1  2|
    let c = a(p(x, y));

    println!("{:#?}", c);
}



//use std::ops::{Index, IndexMut};
//
//#[derive(Debug)]
//struct Array {
//    data: Vec<f32>,
//    shape: Vec<usize>, // Dimensions of the array
//}
//
//impl Array {
//    fn new(shape: Vec<usize>, initial_value: f32) -> Self {
//        let size = shape.iter().product();
//        Array {
//            data: vec![initial_value; size],
//            shape,
//        }
//    }
//
//    // Define your affine transform to compute 1-D index from N-D indices
//    fn affine_transform(&self, nd_indices: &[usize]) -> Option<usize> {
//        if nd_indices.len() != self.shape.len() {
//            return None;
//        }
//
//        // Example affine transform: here you can implement your actual logic
//        let mut idx = 0;
//        for (i, &dim_index) in nd_indices.iter().enumerate() {
//            if dim_index >= self.shape[i] {
//                return None;
//            }
//            idx = idx * self.shape[i] + dim_index; // Simple affine transformation
//        }
//
//        Some(idx)
//    }
//}
//
//// Implement immutable indexing (using `[]`)
//impl Index<&[usize]> for Array {
//    type Output = f32;
//
//    fn index(&self, indices: &[usize]) -> &Self::Output {
//        let idx = self.affine_transform(indices).expect("Invalid index");
//        &self.data[idx]
//    }
//}
//
//// Implement mutable indexing (using `[]`)
//impl IndexMut<&[usize]> for Array {
//    fn index_mut(&mut self, indices: &[usize]) -> &mut Self::Output {
//        let idx = self.affine_transform(indices).expect("Invalid index");
//        &mut self.data[idx]
//    }
//}
