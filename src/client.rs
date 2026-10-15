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

pyo3::create_exception!(ssh2rs, SSHError, PyRuntimeError);
pyo3::create_exception!(ssh2rs, NotConnected, SSHError);
pyo3::create_exception!(ssh2rs, PasswordRequired, SSHError);
pyo3::create_exception!(ssh2rs, AuthenticationRequired, SSHError);
pyo3::create_exception!(ssh2rs, AuthFailed, SSHError);
pyo3::create_exception!(ssh2rs, Connect, SSHError);
pyo3::create_exception!(ssh2rs, Handshake, SSHError);
pyo3::create_exception!(ssh2rs, SSH2, SSHError);
pyo3::create_exception!(ssh2rs, Command, SSHError);
pyo3::create_exception!(ssh2rs, NoHome, SSHError);
pyo3::create_exception!(ssh2rs, NoKnownHostsFile, SSHError);
pyo3::create_exception!(ssh2rs, NoHostKey, SSHError);
pyo3::create_exception!(ssh2rs, HostKeyChanged, SSHError);
pyo3::create_exception!(ssh2rs, HostUnknown, SSHError);
pyo3::create_exception!(ssh2rs, HostKeyCheckFailed, SSHError);

impl From<SSHClientError> for PyErr {
    fn from(err: SSHClientError) -> PyErr {
        let msg = err.to_string();
        match err {
            SSHClientError::NotConnected => NotConnected::new_err(msg),
            SSHClientError::PasswordRequired => PasswordRequired::new_err(msg),
            SSHClientError::AuthenticationRequired => AuthenticationRequired::new_err(msg),
            SSHClientError::AuthFailed { .. } => AuthFailed::new_err(msg),
            SSHClientError::Connect { .. } => Connect::new_err(msg),
            SSHClientError::Handshake { .. } => Handshake::new_err(msg),
            SSHClientError::SSH2 { .. } => SSH2::new_err(msg),
            SSHClientError::Command { .. } => Command::new_err(msg),
            SSHClientError::NoHome => NoHome::new_err(msg),
            SSHClientError::NoKnownHostsFile => NoKnownHostsFile::new_err(msg),
            SSHClientError::NoHostKey => NoHostKey::new_err(msg),
            SSHClientError::HostKeyChanged { .. } => HostKeyChanged::new_err(msg),
            SSHClientError::HostUnknown { .. } => HostUnknown::new_err(msg),
            SSHClientError::HostKeyCheckFailed { .. } => HostKeyCheckFailed::new_err(msg),
        }
    }
}

#[pyclass]
pub struct SSHResult {
    #[pyo3(get)]
    pub output: String,
    #[pyo3(get)]
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
