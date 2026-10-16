use std::path::PathBuf;

use russh::{
    client,
    keys::{PublicKeyOrCertificate, known_hosts},
};

use crate::{error::SSHClientError, policy::HostKeyPolicy};

pub struct Handler {
    pub host: String,
    pub port: u16,
    pub policy: HostKeyPolicy,
    pub known_hosts_file_path: Option<PathBuf>,
}

impl client::Handler for Handler {
    type Error = SSHClientError;

    async fn check_server_key(&mut self, key: &PublicKeyOrCertificate) -> Result<bool, Self::Error> {
        if self.policy == HostKeyPolicy::Disabled {
            return Ok(true);
        }
        let path = self.known_hosts_file_path.as_ref().ok_or(SSHClientError::KnownHostsPathRequired)?;
        let key = key.public_key();
        match known_hosts::check_known_hosts_path(&self.host, self.port, &key, path) {
            Ok(true) => Ok(true),
            Ok(false) if self.policy == HostKeyPolicy::AcceptNew => {
                known_hosts::learn_known_hosts_path(&self.host, self.port, &key, path)?;
                Ok(true)
            }
            Ok(false) => Err(SSHClientError::HostKeyUnknown(self.host.clone())),
            Err(russh::keys::Error::KeyChanged { .. }) => Err(SSHClientError::HostKeyChanged(self.host.clone())),
            Err(e) => Err(e.into()),
        }
    }
}
