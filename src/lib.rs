use pyo3::prelude::*;

pub mod client;
pub mod core;
pub mod error;
pub mod handler;
pub mod policy;

#[pymodule]
mod ssh2rs {
    #[pymodule_export]
    use super::client::{SSHClient, SSHResult};

    #[pymodule_export]
    use super::error::{
        AuthFailed, AuthenticationRequired, Command, Connect, Handshake, HostKey, HostKeyChanged, HostKeyCheckFailed, HostUnknown, NoHome,
        NotConnected, PasswordRequired, Russh, SSH2, SSHError,
    };
}
