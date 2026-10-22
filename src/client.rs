use std::{sync::Arc, time::Duration};

use pyo3::{Bound, Py, PyAny, PyErr, PyResult, Python, exceptions::PyValueError, pyclass, pymethods};
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
    #[pyo3(signature = (host, port, timeout = Some(30.0), host_key_policy = HostKeyPolicy::Strict))]
    fn connect(py: Python<'_>, host: String, port: u16, timeout: Option<f64>, host_key_policy: HostKeyPolicy) -> PyResult<Self> {
        let duration = to_duration(timeout)?;
        let core = py.detach(|| get_runtime().block_on(SSHCore::connect(host, port, duration, host_key_policy)))?;
        Ok(Self { core: Arc::new(core) })
    }

    fn authenticate(&self, py: Python<'_>, username: String, password: Option<String>) -> PyResult<()> {
        Ok(py.detach(|| get_runtime().block_on(self.core.authenticate(&username, password.as_deref())))?)
    }

    fn exec(&self, py: Python<'_>, cmd: String, timeout: Option<f64>) -> PyResult<SSHResult> {
        let duration = to_duration(timeout)?;
        Ok(py.detach(|| get_runtime().block_on(self.core.exec(&cmd, duration)))?)
    }

    #[getter]
    fn latest_result(&self, py: Python<'_>) -> PyResult<SSHResult> {
        Ok(py.detach(|| get_runtime().block_on(self.core.latest_result())))
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
    #[pyo3(signature = (host, port, timeout = Some(30.0), host_key_policy = HostKeyPolicy::Strict))]
    fn connect<'py>(py: Python<'py>, host: String, port: u16, timeout: Option<f64>, host_key_policy: HostKeyPolicy) -> PyResult<Bound<'py, PyAny>> {
        let duration = to_duration(timeout)?;
        future_into_py(py, async move {
            SSHCore::connect(host, port, duration, host_key_policy)
                .await
                .map(|core| AsyncSSHClient { core: Arc::new(core) })
                .map_err(PyErr::from)
        })
    }

    fn authenticate<'py>(&self, py: Python<'py>, username: String, password: Option<String>) -> PyResult<Bound<'py, PyAny>> {
        let core = self.core.clone();
        future_into_py(
            py,
            async move { core.authenticate(&username, password.as_deref()).await.map_err(PyErr::from) },
        )
    }

    fn exec<'py>(&self, py: Python<'py>, cmd: String, timeout: Option<f64>) -> PyResult<Bound<'py, PyAny>> {
        let core = self.core.clone();
        let duration = to_duration(timeout)?;
        future_into_py(py, async move { core.exec(&cmd, duration).await.map_err(PyErr::from) })
    }

    #[getter]
    fn latest_result(&self, py: Python<'_>) -> PyResult<SSHResult> {
        Ok(py.detach(|| get_runtime().block_on(self.core.latest_result())))
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

fn to_duration(secs: Option<f64>) -> PyResult<Option<Duration>> {
    secs.map(|s| Duration::try_from_secs_f64(s).map_err(|e| PyValueError::new_err(e.to_string())))
        .transpose()
}
