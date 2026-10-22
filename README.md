<p>
    <h1 align="center">ssh2rs</h1>
    <p align="center">
        <img src="https://badgen.net/badge/python/3.10/green">
        <img src="https://badgen.net/badge/python/3.11/green">
        <img src="https://badgen.net/badge/python/3.12/green">
        <img src="https://badgen.net/badge/python/3.13/green">
        <img src="https://badgen.net/badge/python/3.14/green">
    </p>
    <p align="center">
        <img src="https://badgen.net/badge/os/linux/blue">
        <img src="https://badgen.net/badge/os/windows/blue">
        <img src="https://badgen.net/badge/os/macos/blue">
    </p>
    <p align="center">
        ssh2rs is an easy and fast SSH2 client for Python, written in Rust. <br>
        Run commands on remote hosts with a sync or async API.
    </p>
</p>

## Available features

- Sync client (`SSHClient`) and async client (`AsyncSSHClient`).
- Run remote commands and get `stdout`, `stderr` and exit code (`rc`).
- Read output of the last command through `latest_result`, including a timed-out command.
- Password authentication, plus SSH agent and default key files (`~/.ssh`).
- Host key verification through `known_hosts` (`Strict` by default).
- Context managers (`with` / `async with`) disconnect automatically.
- Connect and exec timeouts in seconds.
- Typed errors (`SSHError` and subclasses) for connect, auth, command and timeout failures.

## Sync example

```python
from ssh2rs import SSHClient

with SSHClient.connect("example.com", 22) as client:
    client.authenticate("user", "password")
    result = client.exec("uname -a")
    print(result.stdout, result.stderr, result.rc)
```

## Async example

```python
import asyncio

from ssh2rs import AsyncSSHClient


async def main():
    async with await AsyncSSHClient.connect("example.com", 22) as client:
        await client.authenticate("user", "password")
        result = await client.exec("uname -a")
        print(result.stdout, result.stderr, result.rc)


asyncio.run(main())
```

## Authentication

`authenticate(username, password=None)` tries methods in this order and stops at the first success:

1. SSH agent keys (`SSH_AUTH_SOCK`).
2. Default key files in `~/.ssh`: `id_ed25519`, `id_ecdsa`, `id_rsa`. Keys with a passphrase are skipped.
3. Password, only if `password` is given.

If no method succeeds, `authenticate` raises `PasswordRequired` when `password` is `None`, otherwise `AuthenticationFailed`.

```python
client.authenticate("user")                # agent and key files only
client.authenticate("user", "password")    # agent, key files, then password
```

## Host key policy

Pass the policy as the `host_key_policy` argument of `connect`. Default is `HostKeyPolicy.Strict`.


| Policy                    | Unknown host           | Changed host key | Use case                      |
|---------------------------|------------------------|------------------|-------------------------------|
| `HostKeyPolicy.Strict`    | Rejected               | Rejected         | Default, production           |
| `HostKeyPolicy.AcceptNew` | Added to `known_hosts` | Rejected         | First connection to new hosts |
| `HostKeyPolicy.Disabled`  | Accepted               | Accepted         | Testing only, no MITM check   |

**Warning:** `HostKeyPolicy.Disabled` skips host key verification. Anyone on the network path can impersonate the server.

```python
from ssh2rs import HostKeyPolicy, SSHClient

client = SSHClient.connect("example.com", 22, host_key_policy=HostKeyPolicy.AcceptNew)
```

## Timeouts

Both timeouts are in seconds (`float`). Pass `None` to disable one.

| Method    | Argument  | Default | Limits                          |
|-----------|-----------|---------|---------------------------------|
| `connect` | `timeout` | `30.0`  | TCP connect and SSH handshake   |
| `exec`    | `timeout` | `None`  | Remote command execution        |

```python
client = SSHClient.connect("example.com", 22, timeout=10.0)
result = client.exec("sleep 5", timeout=2.0)  # raises ssh2rs.Timeout
```

A timed-out call raises `ssh2rs.Timeout`.

## Latest result

`latest_result` is a read-only property on both `SSHClient` and `AsyncSSHClient`. It returns an `SSHResult` with the `stdout` and `stderr` of the last `exec` call.

**Note:** `latest_result` also works when `exec` timed out. It returns the output collected before the timeout, so you can inspect partial output after `ssh2rs.Timeout`.

```python
from ssh2rs import SSHClient, Timeout

with SSHClient.connect("example.com", 22) as client:
    client.authenticate("user", "password")
    try:
        client.exec("sleep 5; echo done", timeout=2.0)
    except Timeout:
        print(client.latest_result.stdout)  # partial output before timeout
```

- Each `exec` call resets the stored output. Before the first `exec`, both fields are empty strings.
- `rc` is the exit code of the last command, or `None` if the command did not report one. After a timeout, `rc` is usually `None` because the command did not finish.

## Install

### From source

Requires [uv](https://docs.astral.sh/uv/) and a Rust toolchain ([rustup](https://rustup.rs/)). The build uses [maturin](https://www.maturin.rs/).

```sh
    git clone https://github.com/pchomik/ssh2rs
    cd ssh2rs
    uv sync
```

### From source for testing

```sh
    cd ssh2rs
    uv sync --extra test
    source .env && uv run pytest -v
```

### From source for development

```sh
    cd ssh2rs
    uv sync --all-extras
```

### Rebuild after changes

After you modify the Rust code in `src/`, rebuild the binary:

```sh
    uv sync --reinstall-package ssh2rs
```

Tests need a reachable SSH server set in `.env` (`SSH2RS_TEST_HOST`, `SSH2RS_TEST_PORT`, `SSH2RS_TEST_USERNAME`, optional `SSH2RS_TEST_PASSWORD`).

## Contribution

Please feel free to present your idea by code example (pull request) or reported issues.

**Warning:** before opening an issue or pull request, first create a discussion in the [Discussions](https://github.com/pchomik/ssh2rs/discussions) tab. Describe the idea or problem there first.

- Issues: [github.com/pchomik/ssh2rs/issues](https://github.com/pchomik/ssh2rs/issues)
- Repository: [github.com/pchomik/ssh2rs](https://github.com/pchomik/ssh2rs)

## License

MIT. See [LICENSE](LICENSE).
