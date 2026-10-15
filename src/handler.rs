use russh::{
    client,
    keys::{PublicKeyOrCertificate, known_hosts},
};

use crate::{error::SSHClientError, policy::HostKeyPolicy};

pub struct Handler {
    host: String,
    port: u16,
    policy: HostKeyPolicy,
}

impl client::Handler for Handler {
    type Error = SSHClientError;

    async fn check_server_key(&mut self, key: &PublicKeyOrCertificate) -> Result<bool, Self::Error> {
        if self.policy == HostKeyPolicy::Disabled {
            return Ok(true);
        }
        let key = key.public_key();
        match known_hosts::check_known_hosts(&self.host, self.port, &key) {
            Ok(true) => Ok(true),
            Ok(false) if self.policy == HostKeyPolicy::AcceptNew => {
                known_hosts::learn_known_hosts(&self.host, self.port, &key)?;
                Ok(true)
            }
            Ok(false) => Err(SSHClientError::HostUnknown(self.host.clone())),
            Err(russh::keys::Error::KeyChanged { .. }) => Err(SSHClientError::HostKeyChanged(self.host.clone())),
            Err(e) => Err(e.into()),
        }
    }
}
