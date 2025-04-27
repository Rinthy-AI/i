use pyo3::prelude::*;
use pyo3::types::PyList;

#[pyclass]
struct Tensor {
    data: Vec<f32>,
    shape: Vec<usize>,
}

fn infer_shape(list: &Bound<'_, PyList>) -> PyResult<Vec<usize>> {
    let mut shape = Vec::new();
    let mut current = list.clone();

    loop {
        shape.push(current.len());

        if current.is_empty() {
            break;
        }

        let first_item = current.get_item(0)?;
        match first_item.downcast::<PyList>() {
            Ok(sublist) => current = sublist.clone(),
            Err(_) => break,
        }
    }

    Ok(shape)
}

fn validate_and_flatten(
    list: &Bound<'_, PyList>,
    shape: &[usize],
    dim: usize,
    data: &mut Vec<f32>,
) -> PyResult<()> {
    if dim >= shape.len() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Array has more dimensions than expected",
        ));
    }

    if list.len() != shape[dim] {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Inconsistent shape: Expected {} elements at dimension {}, got {}",
            shape[dim],
            dim,
            list.len()
        )));
    }

    if dim == shape.len() - 1 {
        for element in list.iter() {
            let element = element.extract()?;
            data.push(element);
        }
    } else {
        for element in list.iter() {
            let sublist = element.downcast::<PyList>()?;
            validate_and_flatten(&sublist, shape, dim + 1, data)?;
        }
    }

    Ok(())
}

#[pymethods]
impl Tensor {
    #[new]
    fn new(elements: &Bound<'_, PyList>) -> PyResult<Self> {
        let shape = infer_shape(elements)?;
        let mut data = Vec::new();
        validate_and_flatten(elements, &shape, 0, &mut data)?;

        let expected_size: usize = shape.iter().product();
        if data.len() != expected_size {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Data size {} does not match shape {:?} (expected {})",
                data.len(),
                shape,
                expected_size
            )));
        }

        Ok(Self { data, shape })
    }

    fn __str__(&self) -> PyResult<String> {
        Ok(format!(
            "Tensor(shape={:?}, data={:?})",
            self.shape, self.data
        ))
    }
}

#[pymodule]
fn itensor(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Tensor>()?;
    Ok(())
}
