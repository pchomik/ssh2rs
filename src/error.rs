use std::{io, path::PathBuf};

use pyo3::{PyErr, exceptions::PyRuntimeError};

#[derive(Debug, thiserror::Error)]
pub enum SSHClientError {
    #[error("connected client required")]
    NotConnected,
    #[error("password not provided")]
    PasswordRequired,
    #[error("authenticated client required")]
    AuthenticationRequired,
    #[error("authentication failed")]
    AuthenticationFailed,
    #[error("password auth failed")]
    AuthFailed(ssh2::Error),
    #[error("failed to connect to {addr}:{port}")]
    Connect {
        addr: String,
        port: u16,
        #[source]
        source: std::io::Error,
    },
    #[error("SSH handshake failed")]
    Handshake(#[source] ssh2::Error),
    #[error("{0}")]
    Command(&'static str),
    #[error("ssh2 related error")]
    SSH2(#[from] ssh2::Error),
    #[error("no access to home directory")]
    NoHome,
    #[error("key for host {0} changed")]
    HostKeyChanged(String),
    #[error("key for host {0} unknown")]
    HostKeyUnknown(String),
    #[error("incorrect key for host {0}")]
    HostKeyCheckFailed(String),
    #[error("russh related error")]
    Russh(#[from] russh::Error),
    #[error("host key error")]
    HostKey(#[from] russh::keys::Error),
    #[error("cannot read known_hosts file")]
    KnownHostsUnreadable { path: PathBuf, source: io::Error },
    #[error("path to known_hosts was not set")]
    KnownHostsPathRequired,
}

pyo3::create_exception!(ssh2rs, SSHError, PyRuntimeError);
pyo3::create_exception!(ssh2rs, NotConnected, SSHError);
pyo3::create_exception!(ssh2rs, PasswordRequired, SSHError);
pyo3::create_exception!(ssh2rs, AuthenticationRequired, SSHError);
pyo3::create_exception!(ssh2rs, AuthenticationFailed, SSHError);
pyo3::create_exception!(ssh2rs, AuthFailed, SSHError);
pyo3::create_exception!(ssh2rs, Connect, SSHError);
pyo3::create_exception!(ssh2rs, Handshake, SSHError);
pyo3::create_exception!(ssh2rs, SSH2, SSHError);
pyo3::create_exception!(ssh2rs, Russh, SSHError);
pyo3::create_exception!(ssh2rs, HostKey, SSHError);
pyo3::create_exception!(ssh2rs, Command, SSHError);
pyo3::create_exception!(ssh2rs, NoHome, SSHError);
pyo3::create_exception!(ssh2rs, HostKeyChanged, SSHError);
pyo3::create_exception!(ssh2rs, HostKeyUnknown, SSHError);
pyo3::create_exception!(ssh2rs, HostKeyCheckFailed, SSHError);
pyo3::create_exception!(ssh2rs, KnownHostsUnreadable, SSHError);
pyo3::create_exception!(ssh2rs, KnownHostsPathRequired, SSHError);

impl From<SSHClientError> for PyErr {
    fn from(err: SSHClientError) -> PyErr {
        let msg = err.to_string();
        match err {
            SSHClientError::NotConnected => NotConnected::new_err(msg),
            SSHClientError::PasswordRequired => PasswordRequired::new_err(msg),
            SSHClientError::AuthenticationRequired => AuthenticationRequired::new_err(msg),
            SSHClientError::AuthenticationFailed => AuthenticationFailed::new_err(msg),
            SSHClientError::AuthFailed { .. } => AuthFailed::new_err(msg),
            SSHClientError::Connect { .. } => Connect::new_err(msg),
            SSHClientError::Handshake { .. } => Handshake::new_err(msg),
            SSHClientError::SSH2 { .. } => SSH2::new_err(msg),
            SSHClientError::Command { .. } => Command::new_err(msg),
            SSHClientError::NoHome => NoHome::new_err(msg),
            SSHClientError::Russh { .. } => Russh::new_err(msg),
            SSHClientError::HostKey { .. } => HostKey::new_err(msg),
            SSHClientError::KnownHostsUnreadable { .. } => KnownHostsUnreadable::new_err(msg),
            SSHClientError::KnownHostsPathRequired => KnownHostsPathRequired::new_err(msg),
            SSHClientError::HostKeyChanged { .. } => HostKeyChanged::new_err(msg),
            SSHClientError::HostKeyUnknown { .. } => HostKeyUnknown::new_err(msg),
            SSHClientError::HostKeyCheckFailed { .. } => HostKeyCheckFailed::new_err(msg),
        }
    }
}
