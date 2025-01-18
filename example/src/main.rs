use macros::i;

fn main() {
    // matrix multiplication kernel
    let mm = i!(
        m: ik*kj~ijk
        a: +ijk~ij
        m.a
    );

    // i operates on contiguous arrays
    let x = vec![0., -1., 1., 0.];
    let y = vec![1., 2., 3., 4.];
    let mut out = vec![0., 0., 0., 0.];

    // matmul
    // |0 -1||1 2| = |0 0| + |-3 -4| = |-3 -4|
    // |1  0||3 4|   |1 2|   | 0  0|   | 1  2|

    // pass dims with data vecs
    let c = mm(&x, 2, 2, &y, 2, 2, &mut out, 2, 2);

    println!("{:#?}", out);
}
