use pyo3::prelude::*;

pub mod client;

#[pymodule]
mod ssh2rs {
    #[pymodule_export]
    use super::client::{
        AuthFailed, AuthenticationRequired, Command, Connect, Handshake, HostKeyChanged, HostKeyCheckFailed, HostUnknown, NoHome, NoHostKey,
        NoKnownHostsFile, NotConnected, PasswordRequired, SSH2, SSHClient, SSHError, SSHResult,
    };
}
