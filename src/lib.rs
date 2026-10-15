use pyo3::prelude::*;

pub mod client;

#[pymodule]
mod ssh2rs {
    use pyo3::prelude::*;

    #[pymodule_export]
    use super::client::{SSHClient, SSHResult};
    }
}
