pub mod client;

#[pyo3::pymodule]
mod ssh2rs {
    #[pymodule_export]
    use super::client::{SSHClient, SSHResult};
}
