use pyo3::pyclass;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[pyclass(from_py_object)]
pub enum HostKeyPolicy {
    Disabled,
    AcceptNew,
    #[default]
    Scrict,
}
