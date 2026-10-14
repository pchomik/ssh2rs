use pyo3::PyErr;
use pyo3::exceptions::PyRuntimeError;
use pyo3::{pyclass, pymethods};
use ssh2::{CheckResult, ExtendedData, Session};
use std::io::Read;
use std::net::TcpStream;

#[derive(Debug, thiserror::Error)]
pub enum SSHClientError {
    #[error("connected client required")]
    NotConnected,
    #[error("password not provided")]
    PasswordRequired,
    #[error("authentication required")]
    AuthenticationRequired,
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
    #[error("no known_hosts file")]
    NoKnownHostsFile,
    #[error("host not available in known_hosts")]
    NoHostKey,
    #[error("key for host {0} changed")]
    HostKeyChanged(String),
    #[error("key for host {0} unknown")]
    HostUnknown(String),
    #[error("incorrect key for host {0}")]
    HostKeyCheckFailed(String),
}

impl From<SSHClientError> for PyErr {
    fn from(err: SSHClientError) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}

#[pyclass]
pub struct SSHResult {
    pub output: String,
    pub rc: i32,
}

#[pyclass]
pub struct SSHClient {
    addr: String,
    port: u16,
    username: String,
    password: Option<String>,
    authenticated: bool,
    session: Session,
}

#[pymethods]
impl SSHClient {
    #[new]
    pub fn connect(addr: String, port: u16, username: String, password: Option<String>) -> Result<Self, SSHClientError> {
        let tcp = TcpStream::connect((addr.clone(), port)).map_err(|source| SSHClientError::Connect {
            addr: addr.clone(),
            port,
            source,
        })?;
        let mut session = Session::new().map_err(|source| SSHClientError::Connect {
            addr: addr.clone(),
            port,
            source: source.into(),
        })?;
        session.set_tcp_stream(tcp);
        session.handshake().map_err(SSHClientError::Handshake)?;
        Ok(SSHClient {
            addr,
            port,
            username,
            password,
            authenticated: false,
            session,
        })
    }

    pub fn authenticate(&mut self) -> Result<(), SSHClientError> {
        let _ = self.session.userauth_agent(&self.username);

        if !self.session.authenticated() {
            let password = self.password.as_ref().ok_or(SSHClientError::PasswordRequired)?;
            self.session
                .userauth_password(&self.username, password)
                .map_err(SSHClientError::AuthFailed)?;
        }

        if !self.session.authenticated() {
            return Err(SSHClientError::PasswordRequired);
        }

        self.authenticated = true;
        Ok(())
    }

    pub fn check_known_hosts(&self) -> Result<(), SSHClientError> {
        let mut known = self.session.known_hosts().map_err(SSHClientError::SSH2)?;
        let home = std::env::home_dir().ok_or(SSHClientError::NoHome)?;
        let file = home.join(".ssh/known_hosts");
        if !file.exists() {
            return Err(SSHClientError::NoKnownHostsFile);
        }

        known.read_file(&file, ssh2::KnownHostFileKind::OpenSSH)?;
        let (key, _) = self.session.host_key().ok_or(SSHClientError::NoHostKey)?;

        let host = self.addr.to_string();
        match known.check_port(&self.addr, self.port, key) {
            CheckResult::Match => Ok(()),
            CheckResult::Mismatch => Err(SSHClientError::HostKeyChanged(host)),
            CheckResult::NotFound => Err(SSHClientError::HostUnknown(host)),
            CheckResult::Failure => Err(SSHClientError::HostKeyCheckFailed(host)),
        }
    }

    pub fn exec(&self, cmd: String) -> Result<SSHResult, SSHClientError> {
        if !self.authenticated {
            return Err(SSHClientError::AuthenticationRequired);
        }

        self.session.set_timeout(30_000);

        let mut channel = self
            .session
            .channel_session()
            .map_err(|_| SSHClientError::Command("failed to establish channel"))?;
        channel.handle_extended_data(ExtendedData::Merge).map_err(SSHClientError::SSH2)?;
        channel.exec(&cmd).map_err(|_| SSHClientError::Command("failed to start command"))?;

        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .map_err(|_| SSHClientError::Command("failed to read output"))?;
        channel.wait_close().map_err(|_| SSHClientError::Command("failed to close channel"))?;

        let rc = channel.exit_status().map_err(|_| SSHClientError::Command("failed to read exit status"))?;

        Ok(SSHResult { output, rc })
    }
}
