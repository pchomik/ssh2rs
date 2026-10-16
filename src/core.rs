use std::{path::PathBuf, sync::Arc, time::Duration};

use russh::{
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

pub struct SSHCore {
    session: Mutex<Handle<Handler>>,
}

impl SSHCore {
    pub async fn connect(addr: String, port: u16, policy: HostKeyPolicy) -> Result<Self, SSHClientError> {
        let config = Config {
            inactivity_timeout: Some(Duration::from_secs(30)),
            ..Config::default()
        };

        let known_hosts_file_path = known_hosts_path(policy)?;

        let handler = Handler {
            host: addr.clone(),
            port,
            policy,
            known_hosts_file_path,
        };

        let handle = client::connect(Arc::new(config), (addr.as_str(), port), handler).await?;
        Ok(SSHCore { session: Mutex::new(handle) })
    }
    pub async fn authenticate(&mut self, user: &str, password: Option<&str>) -> Result<(), SSHClientError> {
        let home = std::env::home_dir().ok_or(SSHClientError::NoHome)?;

        let hash_alg = self.session.get_mut().best_supported_rsa_hash().await?.flatten();

        if let Ok(mut agent) = AgentClient::connect_env().await {
            for id in agent.request_identities().await? {
                let AgentIdentity::PublicKey { key, .. } = id else { continue };
                let Ok(res) = self.session.get_mut().authenticate_publickey_with(user, key, hash_alg, &mut agent).await else {
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
            let res = self
                .session
                .get_mut()
                .authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg))
                .await?;
            if res.success() {
                return Ok(());
            }
        }

        if password.is_none() {
            return Err(SSHClientError::PasswordRequired);
        }
        let res = self.session.get_mut().authenticate_password(user, password.unwrap()).await?;

        if res.success() {
            return Ok(());
        }

        Err(SSHClientError::AuthenticationFailed)
    }
    pub async fn exec() {}
    pub async fn disconnect() {}
}
