use pyo3::prelude::*;

pub mod client;
pub mod core;
pub mod error;
pub mod handler;
pub mod policy;

#[pymodule]
mod ssh2rs {
    #[pymodule_export]
    use super::client::{AsyncSSHClient, SSHClient, SSHResult};

    #[pymodule_export]
    use super::policy::HostKeyPolicy;

    #[pymodule_export]
    use super::error::{
        Agent, AuthenticationFailed, AuthenticationRequired, Command, Connect, ConnectionLost, Handshake, HostKey, HostKeyChanged, HostKeyUnknown,
        KnownHostsPathRequired, KnownHostsUnreadable, NoHome, NotConnected, PasswordRequired, SSH2, SSHError,
    };
}
