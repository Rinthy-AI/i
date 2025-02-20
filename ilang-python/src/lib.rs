use std::ffi::c_void;

use compiler::{
    backend::{block::BlockBackend, rust::RustBackend, Build, Render},
    block::{Block, Statement, Type as ILangType},
    graph::Graph,
    grapher,
    lowerer::Lowerer,
    parser::Parser,
};
use libffi::{
    low::CodePtr,
    middle::{arg, Cif, Type as CifType},
};
use pyo3::{
    prelude::*,
    types::{PyList, PyTuple},
};

fn ilang_type_to_cif_type(type_: &ILangType) -> CifType {
    let slice = CifType::structure(vec![CifType::usize(), CifType::usize()].into_iter());
    match type_ {
        ILangType::Int(_) => CifType::usize(),
        ILangType::Array(_) | ILangType::ArrayRef(_) => slice,
    }
}

#[pyclass]
#[derive(Debug)]
struct Component {
    _src: String,
    graph: Graph,
}

#[derive(Debug, Clone, FromPyObject, IntoPyObject)]
#[pyo3(transparent)]
struct ArrayPointer(usize);

impl ArrayPointer {
    unsafe fn as_ptr(&self) -> *const f32 {
        self.0 as *const f32
    }
    unsafe fn as_mut_ptr(&self) -> *mut f32 {
        self.0 as *mut f32
    }
    fn from_ptr(ptr: *const f32) -> Self {
        ArrayPointer(ptr as usize)
    }
}

#[pyclass]
#[derive(Debug, FromPyObject)]
#[repr(C)]
struct Tensor {
    #[pyo3(get)]
    ptr: ArrayPointer,
    #[pyo3(get)]
    dims: Vec<usize>,
}

#[pymethods]
impl Tensor {
    #[new]
    fn py_new(data_list: &Bound<PyList>, shape: &Bound<PyTuple>) -> PyResult<Self> {
        let data: Vec<f32> = data_list.extract()?;
        let ptr = ArrayPointer::from_ptr(data.as_ptr());
        std::mem::forget(data);
        let dims = shape.extract()?;
        Ok(Tensor { ptr, dims })
    }
    fn __repr__(&self) -> String {
        let data =
            unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.dims.iter().product()) };
        format!("Tensor {{ data: {:?}, shape: {:?} }}", data, self.dims)
    }
    fn __del__(&self) {
        let _data = unsafe {
            Vec::from_raw_parts(
                self.ptr.as_mut_ptr(),
                self.dims.iter().product(),
                self.dims.iter().product(),
            )
        };
    }
}

impl TryFrom<String> for Component {
    type Error = PyErr;
    fn try_from(src: String) -> Result<Self, Self::Error> {
        // TODO better error handling here
        let (_ast, expr_bank) = Parser::new(&src).unwrap().parse().unwrap();
        let graph = grapher::graph(&expr_bank);
        Ok(Component { _src: src, graph })
    }
}

#[pymethods]
impl Component {
    #[new]
    fn py_new(src: String) -> PyResult<Self> {
        src.try_into()
    }
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
    fn as_block(&self) -> String {
        let block = Lowerer::new().lower(&self.graph);
        BlockBackend::render(&block)
    }
    #[pyo3(signature = (*args))]
    fn realize(&self, args: &Bound<'_, PyTuple>) -> PyResult<()> {
        let tensors: Vec<Tensor> = args.extract()?;
        let data: Vec<&Tensor> = tensors.iter().collect();
        let block = Lowerer::new().lower(&self.graph);
        run_rust_impl(&block, &data)
    }
}

fn run_rust_impl(schedule: &Block, data: &[&Tensor]) -> PyResult<()> {
    // println!("data: {:?}", data);
    let functions = schedule
        .statements
        .iter()
        .filter(|s| match s {
            Statement::Function { .. } => true,
            _ => false,
        })
        .collect::<Vec<&Statement>>();
    if functions.is_empty() {
        // TODO return proper error
        panic!("no functions")
    }
    let exec_func = functions.last().unwrap();
    let mut args = vec![];
    let data_ptrs = data
        .iter()
        .map(|t| unsafe { std::slice::from_raw_parts(t.ptr.as_ptr(), t.dims.iter().product()) })
        .collect::<Vec<&[f32]>>();
    for (ptr_idx, tensor) in data.iter().enumerate() {
        args.push(arg(&data_ptrs[ptr_idx]));
        for dim in tensor.dims.iter() {
            args.push(arg(dim));
        }
    }
    match exec_func {
        Statement::Function {
            ident,
            args: params,
            ..
        } => {
            let param_types = params
                .iter()
                .map(|a| ilang_type_to_cif_type(&a.type_))
                .collect::<Vec<CifType>>();
            let rust = RustBackend::render(&schedule);
            let dylib_path = RustBackend::build(&rust).unwrap();
            let cif = Cif::new(param_types.into_iter(), CifType::void());
            unsafe {
                let lib = libloading::Library::new(dylib_path).unwrap();
                let ilang_run: libloading::Symbol<*const c_void> =
                    lib.get(ident.as_bytes()).unwrap();
                cif.call::<()>(CodePtr(ilang_run.cast_mut()), &args);
            }
            Ok(())
        }
        _ => unreachable!(),
    }
}

#[pymodule]
fn ilang(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Component>()?;
    m.add_class::<Tensor>()?;
    Ok(())
}
