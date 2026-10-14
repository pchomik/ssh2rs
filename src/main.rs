use ssh2rs::client::{SSHClient, SSHClientError};

fn main() -> Result<(), SSHClientError> {
    // Read test credentials
    let host = std::env::var("SSH2RS_TEST_HOST").unwrap();
    let raw_port = std::env::var("SSH2RS_TEST_PORT").unwrap();
    let port: u16 = raw_port.parse().unwrap();
    let username = std::env::var("SSH2RS_TEST_USERNAME").unwrap();
    let raw_password = std::env::var("SSH2RS_TEST_PASSWORD").unwrap();
    let mut password: Option<String> = None;
    if !raw_password.is_empty() {
        password = Some(raw_password);
    }

    // Execute SSH test
    let mut client = SSHClient::connect(host, port, username, password)?;

    match client.check_known_hosts() {
        Ok(_) | Err(SSHClientError::NoHome | SSHClientError::NoHostKey | SSHClientError::HostUnknown(_) | SSHClientError::NoKnownHostsFile) => {}
        Err(e) => return Err(e),
    };

    client.authenticate()?;
    let result = client.exec("ls -la".to_string())?;

    println!("=== OUTPUT ===\n{}\n", result.output.trim());
    println!("=== RC ===\n{}\n", result.rc);

    Ok(())
}
