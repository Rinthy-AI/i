use framework::{i, Array};

fn main() -> Result<(), String> {
    let h = i("ik*kj~ijk")?;
    let a = i("+ijk~ij")?;

    let mm = h.chain(&a);

    // todo: make this work
    let x = Array {
        data: vec![0., -1., 1., 0.],
        shape: vec![2, 2],
    };
    let y = Array {
        data: vec![1., 2., 3., 4.],
        shape: vec![2, 2],
    };
    println!("{:#?}", mm(&x, &y));

    Ok(())
}
