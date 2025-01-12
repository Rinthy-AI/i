use framework::{i, Array};

fn main() -> Result<(), String> {
    let h = i("ik*kj~ijk")?;
    let a = i("+ijk~ij")?;

    // this should work EDIT: apparently this won't ever work in stable Rust :(((
    //let mm = a(h);

    let mm = h.chain(&a); // make this work

    // this should work
    //let x = Array { data: vec![0., -1., 1., 0.], shape: vec![2, 2] };
    //let y = Array { data: vec![1., 2., 3., 4.], shape: vec![2, 2] };
    //println!("{:#?}", mm(&x, &y));

    Ok(())
}
