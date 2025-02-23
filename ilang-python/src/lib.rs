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
    exceptions::{PyIndexError, PyValueError},
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
    #[pyo3(get)]
    size: usize,
    #[pyo3(get)]
    capacity: usize,
}

#[pymethods]
impl Tensor {
    #[new]
    fn py_new(data_list: &Bound<PyList>, shape: &Bound<PyTuple>) -> PyResult<Self> {
        let data: Vec<f32> = data_list.extract()?;
        let dims: Vec<usize> = shape.extract()?;
        let size = dims.iter().product();
        // Grab the capacity just in case it doesn't equal size for use in Drop later
        let capacity = data.capacity();

        if data.len() != size {
            return Err(PyErr::new::<PyValueError, _>(
                "'dims' do not match size of 'data'.",
            ));
        }

        let ptr = ArrayPointer::from_ptr(data.as_ptr());
        std::mem::forget(data);

        Ok(Tensor {
            ptr,
            dims,
            size,
            capacity,
        })
    }

    fn __getitem__(&self, index: i32) -> PyResult<f32> {
        if index < 0 || index as usize >= self.size {
            return Err(PyErr::new::<PyIndexError, _>("Index out of bounds."));
        }
        Ok(self.as_slice()[index as usize])
    }

    fn __repr__(&self) -> String {
        format!(
            "Tensor {{ data: {:?}, shape: {:?} }}",
            self.as_slice(),
            self.dims
        )
    }
}

impl Tensor {
    fn as_slice(&self) -> &[f32] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.size) }
    }
}

impl Drop for Tensor {
    fn drop(&mut self) {
        let _ = unsafe { Vec::from_raw_parts(self.ptr.as_mut_ptr(), self.size, self.capacity) };
    }
}

impl TryFrom<String> for Component {
    type Error = PyErr;
    fn try_from(src: String) -> Result<Self, Self::Error> {
        let (_ast, expr_bank) = Parser::new(&src)
            .map_err(|err| PyErr::new::<PyValueError, _>(err))?
            .parse()
            .map_err(|err| PyErr::new::<PyValueError, _>(err.to_string()))?;
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
        let data: Vec<Tensor> = args.extract()?;
        let block = Lowerer::new().lower(&self.graph);
        run_rust_impl(&block, &data)
    }
}

fn run_rust_impl(schedule: &Block, data: &[Tensor]) -> PyResult<()> {
    let exec_func = schedule
        .statements
        .iter()
        .filter(|&s| match s {
            Statement::Function { .. } => true,
            _ => false,
        })
        .last()
        .ok_or_else(|| PyErr::new::<PyValueError, _>("No functions provided."))?;
    let mut args = vec![];
    let data_ptrs = data.iter().map(|t| t.as_slice()).collect::<Vec<&[f32]>>();
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
