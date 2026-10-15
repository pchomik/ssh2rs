use std::path::PathBuf;

use russh::client::Handle;
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
    handle: Mutex<Handle<Handler>>,
}

impl SSHCore {
    pub async fn connect() {}
    pub async fn authenticate() {}
    pub async fn exec() {}
    pub async fn disconnect() {}
}
