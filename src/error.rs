use std::{io, path::PathBuf};

use pyo3::{PyErr, exceptions::PyRuntimeError};

#[derive(Debug, thiserror::Error)]
pub enum SSHClientError {
    #[error("timeout during {0}")]
    Timeout(&'static str),
    #[error("connected client required")]
    NotConnected,
    #[error("password not provided")]
    PasswordRequired,
    #[error("authenticated client required")]
    AuthenticationRequired,
    #[error("wrong username or password/key")]
    AuthenticationFailed,
    #[error("failed to connect to {host}:{port}: {source}")]
    Connect {
        host: String,
        port: u16,
        #[source]
        source: io::Error,
    },
    #[error("SSH handshake failed: {0}")]
    Handshake(#[source] russh::Error),
    #[error("connection lost: {0}")]
    ConnectionLost(#[source] russh::Error),
    #[error("command failed: {0}")]
    Command(String),
    #[error("ssh-agent error: {0}")]
    Agent(#[source] russh::keys::Error),
    #[error("no access to home directory")]
    NoHome,
    #[error("key for host {0} changed")]
    HostKeyChanged(String),
    #[error("key for host {0} unknown")]
    HostKeyUnknown(String),
    #[error("host key error: {0}")]
    HostKey(#[from] russh::keys::Error),
    #[error("SSH protocol error: {0}")]
    SSH2(#[source] russh::Error),
    #[error("cannot read known_hosts file {}", path.display())]
    KnownHostsUnreadable {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("path to known_hosts was not set")]
    KnownHostsPathRequired,
}

impl From<russh::Error> for SSHClientError {
    fn from(e: russh::Error) -> Self {
        use russh::Error as E;
        match e {
            E::SendError => Self::NotConnected,
            E::NotAuthenticated => Self::AuthenticationRequired,
            E::Disconnect | E::HUP | E::ConnectionTimeout | E::InactivityTimeout | E::KeepaliveTimeout | E::IO(_) => Self::ConnectionLost(e),
            E::ChannelOpenFailure(_) | E::RequestDenied => Self::Command(e.to_string()),
            E::Keys(k) => Self::HostKey(k),
            _ => Self::SSH2(e),
        }
    }
}

pyo3::create_exception!(ssh2rs, SSHError, PyRuntimeError);
pyo3::create_exception!(ssh2rs, Timeout, SSHError);
pyo3::create_exception!(ssh2rs, NotConnected, SSHError);
pyo3::create_exception!(ssh2rs, PasswordRequired, SSHError);
pyo3::create_exception!(ssh2rs, AuthenticationRequired, SSHError);
pyo3::create_exception!(ssh2rs, AuthenticationFailed, SSHError);
pyo3::create_exception!(ssh2rs, Connect, SSHError);
pyo3::create_exception!(ssh2rs, Handshake, SSHError);
pyo3::create_exception!(ssh2rs, ConnectionLost, SSHError);
pyo3::create_exception!(ssh2rs, Agent, SSHError);
pyo3::create_exception!(ssh2rs, SSH2, SSHError);
pyo3::create_exception!(ssh2rs, HostKey, SSHError);
pyo3::create_exception!(ssh2rs, Command, SSHError);
pyo3::create_exception!(ssh2rs, NoHome, SSHError);
pyo3::create_exception!(ssh2rs, HostKeyChanged, SSHError);
pyo3::create_exception!(ssh2rs, HostKeyUnknown, SSHError);
pyo3::create_exception!(ssh2rs, KnownHostsUnreadable, SSHError);
pyo3::create_exception!(ssh2rs, KnownHostsPathRequired, SSHError);

impl From<SSHClientError> for PyErr {
    fn from(err: SSHClientError) -> PyErr {
        let msg = err.to_string();
        match err {
            SSHClientError::Timeout(_) => Timeout::new_err(msg),
            SSHClientError::NotConnected => NotConnected::new_err(msg),
            SSHClientError::PasswordRequired => PasswordRequired::new_err(msg),
            SSHClientError::AuthenticationRequired => AuthenticationRequired::new_err(msg),
            SSHClientError::AuthenticationFailed => AuthenticationFailed::new_err(msg),
            SSHClientError::Connect { .. } => Connect::new_err(msg),
            SSHClientError::Handshake(_) => Handshake::new_err(msg),
            SSHClientError::ConnectionLost(_) => ConnectionLost::new_err(msg),
            SSHClientError::Command(_) => Command::new_err(msg),
            SSHClientError::Agent(_) => Agent::new_err(msg),
            SSHClientError::NoHome => NoHome::new_err(msg),
            SSHClientError::SSH2(_) => SSH2::new_err(msg),
            SSHClientError::HostKey(_) => HostKey::new_err(msg),
            SSHClientError::KnownHostsUnreadable { .. } => KnownHostsUnreadable::new_err(msg),
            SSHClientError::KnownHostsPathRequired => KnownHostsPathRequired::new_err(msg),
            SSHClientError::HostKeyChanged(_) => HostKeyChanged::new_err(msg),
            SSHClientError::HostKeyUnknown(_) => HostKeyUnknown::new_err(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn russh_errors_map_to_specific_variants() {
        assert!(matches!(SSHClientError::from(russh::Error::SendError), SSHClientError::NotConnected));
        assert!(matches!(
            SSHClientError::from(russh::Error::NotAuthenticated),
            SSHClientError::AuthenticationRequired
        ));
        assert!(matches!(SSHClientError::from(russh::Error::HUP), SSHClientError::ConnectionLost(_)));
        assert!(matches!(SSHClientError::from(russh::Error::RequestDenied), SSHClientError::Command(_)));
        assert!(matches!(SSHClientError::from(russh::Error::Version), SSHClientError::SSH2(_)));
    }
}
