use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use pyo3::pyclass;
use russh::{
    ChannelMsg, Disconnect,
    client::{self, Config, Handle},
    keys::{
        PrivateKeyWithHashAlg,
        agent::{AgentIdentity, client::AgentClient},
        load_secret_key,
    },
};
use tokio::sync::Mutex;

use crate::{error::SSHClientError, handler::Handler, policy::HostKeyPolicy};

fn known_hosts_path(policy: HostKeyPolicy) -> Result<Option<PathBuf>, SSHClientError> {
    if policy == HostKeyPolicy::Disabled {
        return Ok(None);
    }
    let home = std::env::home_dir().ok_or(SSHClientError::NoHome)?;
    let path = home.join(".ssh/known_hosts");
    if path.exists() {
        std::fs::File::open(&path).map_err(|source| SSHClientError::KnownHostsUnreadable { path: path.clone(), source })?;
    }
    Ok(Some(path))
}

#[derive(Debug)]
#[pyclass]
pub struct SSHResult {
    #[pyo3(get)]
    pub stdout: String,
    #[pyo3(get)]
    pub stderr: String,
    #[pyo3(get)]
    pub rc: Option<u32>,
}

pub struct SSHCore {
    session: Mutex<Handle<Handler>>,
    connected: AtomicBool,
}

impl SSHCore {
    // Explicit disconnect check; russh reports a closed session as ConnectionLost.
    fn ensure_connected(&self) -> Result<(), SSHClientError> {
        if self.connected.load(Ordering::Acquire) {
            return Ok(());
        }
        Err(SSHClientError::NotConnected)
    }

    pub async fn connect(host: String, port: u16, timeout: Option<Duration>, policy: HostKeyPolicy) -> Result<Self, SSHClientError> {
        let config = Config {
            inactivity_timeout: Some(Duration::from_secs(30)),
            ..Config::default()
        };

        let known_hosts_file_path = known_hosts_path(policy)?;

        let handler = Handler {
            host: host.clone(),
            port,
            policy,
            known_hosts_file_path,
        };

        let feature = async {
            let socket = tokio::net::TcpStream::connect((host.as_str(), port))
                .await
                .map_err(|source| SSHClientError::Connect {
                    host: host.clone(),
                    port,
                    source,
                })?;
            client::connect_stream(Arc::new(config), socket, handler).await.map_err(|e| match e {
                SSHClientError::ConnectionLost(e) | SSHClientError::SSH2(e) => SSHClientError::Handshake(e),
                other => other,
            })
        };

        let handle = match timeout {
            Some(d) => tokio::time::timeout(d, feature)
                .await
                .map_err(|_| SSHClientError::Timeout("connect"))
                .and_then(|r| r)?,
            None => feature.await?,
        };

        Ok(SSHCore {
            session: Mutex::new(handle),
            connected: AtomicBool::new(true),
        })
    }
    pub async fn authenticate(&self, username: &str, password: Option<&str>) -> Result<(), SSHClientError> {
        self.ensure_connected()?;
        let home = std::env::home_dir().ok_or(SSHClientError::NoHome)?;
        let mut session = self.session.lock().await;

        let hash_alg = session.best_supported_rsa_hash().await?.flatten();

        #[cfg(unix)]
        let agent = AgentClient::connect_env().await;
        #[cfg(windows)]
        let agent = AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent").await;

        if let Ok(mut agent) = agent {
            for id in agent.request_identities().await.map_err(SSHClientError::Agent)? {
                let AgentIdentity::PublicKey { key, .. } = id else { continue };
                let Ok(res) = session.authenticate_publickey_with(username, key, hash_alg, &mut agent).await else {
                    continue;
                };

                if res.success() {
                    return Ok(());
                }
            }
        }

        for name in ["id_ed25519", "id_ecdsa", "id_rsa"] {
            let Ok(key) = load_secret_key(home.join(".ssh").join(name), None) else {
                continue;
            };
            let res = session
                .authenticate_publickey(username, PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg))
                .await?;
            if res.success() {
                return Ok(());
            }
        }

        if password.is_none() {
            return Err(SSHClientError::PasswordRequired);
        }
        let res = session.authenticate_password(username, password.unwrap()).await?;

        if res.success() {
            return Ok(());
        }

        Err(SSHClientError::AuthenticationFailed)
    }
    pub async fn exec(&self, cmd: &str, timeout: Option<Duration>) -> Result<SSHResult, SSHClientError> {
        self.ensure_connected()?;
        let session = self.session.lock().await;
        let mut channel = session.channel_open_session().await?;
        channel.exec(true, cmd).await?;

        let mut rc = None;
        let mut stdout = vec![];
        let mut stderr = vec![];
        let feature = async {
            loop {
                let Some(msg) = channel.wait().await else {
                    break;
                };

                match msg {
                    ChannelMsg::Data { data } => stdout.extend_from_slice(&data),
                    ChannelMsg::ExtendedData { data, ext: 1 } => stderr.extend_from_slice(&data),
                    ChannelMsg::ExitStatus { exit_status } => rc = Some(exit_status),
                    _ => {}
                }
            }
        };

        match timeout {
            Some(d) => tokio::time::timeout(d, feature)
                .await
                .map_err(|_| SSHClientError::Timeout("execution timeout"))?,
            None => feature.await,
        };
        Ok(SSHResult {
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            rc,
        })
    }
    pub async fn disconnect(&self) -> Result<(), SSHClientError> {
        self.ensure_connected()?;
        let session = self.session.lock().await;
        session.disconnect(Disconnect::ByApplication, "", "English").await?;
        self.connected.store(false, Ordering::Release);
        Ok(())
    }
}
