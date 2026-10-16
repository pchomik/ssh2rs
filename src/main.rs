use ssh2rs::core::SSHCore;
use ssh2rs::error::SSHClientError;
use ssh2rs::policy::HostKeyPolicy;

#[tokio::main]
async fn main() -> Result<(), SSHClientError> {
    // Read test credentials
    let host = std::env::var("SSH2RS_TEST_HOST").unwrap();
    let port: u16 = std::env::var("SSH2RS_TEST_PORT").unwrap().parse().unwrap();
    let username = std::env::var("SSH2RS_TEST_USERNAME").unwrap();
    let password = std::env::var("SSH2RS_TEST_PASSWORD").ok().filter(|p| !p.is_empty());
    let policy = match std::env::var("SSH2RS_TEST_POLICY").as_deref() {
        Ok("disabled") => HostKeyPolicy::Disabled,
        Ok("accept_new") => HostKeyPolicy::AcceptNew,
        _ => HostKeyPolicy::Strict,
    };

    // Execute SSH test: connect, then authenticate
    let mut core = SSHCore::connect(host, port, policy).await?;
    core.authenticate(&username, password.as_deref()).await?;
    println!("authenticated as {username}");

    Ok(())
}
