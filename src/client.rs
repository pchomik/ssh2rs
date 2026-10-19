use std::sync::Arc;

use pyo3::{Bound, Py, PyAny, PyErr, PyResult, Python, pyclass, pymethods};
use pyo3_async_runtimes::tokio::{future_into_py, get_runtime};

use crate::error::SSHClientError;
pub use crate::{
    core::{SSHCore, SSHResult},
    policy::HostKeyPolicy,
};

#[pyclass]
pub struct SSHClient {
    core: Arc<SSHCore>,
}

#[pymethods]
impl SSHClient {
    #[staticmethod]
    #[pyo3(signature = (addr, port, host_key_policy = HostKeyPolicy::Strict))]
    fn connect(py: Python<'_>, addr: String, port: u16, host_key_policy: HostKeyPolicy) -> PyResult<Self> {
        let core = py.detach(|| get_runtime().block_on(SSHCore::connect(addr, port, host_key_policy)))?;
        Ok(Self { core: Arc::new(core) })
    }

    fn authenticate(&self, py: Python<'_>, user: String, password: Option<String>) -> PyResult<()> {
        Ok(py.detach(|| get_runtime().block_on(self.core.authenticate(&user, password.as_deref())))?)
    }

    fn exec(&self, py: Python<'_>, cmd: String) -> PyResult<SSHResult> {
        Ok(py.detach(|| get_runtime().block_on(self.core.exec(&cmd)))?)
    }

    fn disconnect(&self, py: Python<'_>) -> PyResult<()> {
        Ok(py.detach(|| get_runtime().block_on(self.core.disconnect()))?)
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __exit__(&self, py: Python<'_>, _t: Py<PyAny>, _v: Py<PyAny>, _tb: Py<PyAny>) -> PyResult<()> {
        match py.detach(|| get_runtime().block_on(self.core.disconnect())) {
            Err(SSHClientError::NotConnected) | Ok(()) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[pyclass]
pub struct AsyncSSHClient {
    core: Arc<SSHCore>,
}

#[pymethods]
impl AsyncSSHClient {
    #[staticmethod]
    #[pyo3(signature = (addr, port, host_key_policy = HostKeyPolicy::Strict))]
    fn connect<'py>(py: Python<'py>, addr: String, port: u16, host_key_policy: HostKeyPolicy) -> PyResult<Bound<'py, PyAny>> {
        future_into_py(py, async move {
            SSHCore::connect(addr, port, host_key_policy)
                .await
                .map(|core| AsyncSSHClient { core: Arc::new(core) })
                .map_err(PyErr::from)
        })
    }

    fn authenticate<'py>(&self, py: Python<'py>, user: String, password: Option<String>) -> PyResult<Bound<'py, PyAny>> {
        let core = self.core.clone();
        future_into_py(
            py,
            async move { core.authenticate(&user, password.as_deref()).await.map_err(PyErr::from) },
        )
    }

    fn exec<'py>(&self, py: Python<'py>, cmd: String) -> PyResult<Bound<'py, PyAny>> {
        let core = self.core.clone();
        future_into_py(py, async move { core.exec(&cmd).await.map_err(PyErr::from) })
    }

    fn disconnect<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let core = self.core.clone();
        future_into_py(py, async move { core.disconnect().await.map_err(PyErr::from) })
    }

    fn __aenter__<'py>(slf: Py<Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        future_into_py(py, async move { Ok::<_, PyErr>(slf) })
    }

    fn __aexit__<'py>(&self, py: Python<'py>, _t: Py<PyAny>, _v: Py<PyAny>, _tb: Py<PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let core = self.core.clone();
        future_into_py(py, async move {
            match core.disconnect().await {
                Err(SSHClientError::NotConnected) | Ok(()) => Ok(()),
                Err(e) => Err(PyErr::from(e)),
            }
        })
    }
}
