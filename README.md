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
        ssh2rs is an easy and fast SSH2 client for Python, written in Rust. <br>
        Run commands on remote hosts with a sync or async API.
    </p>
</p>

## Available features

- Sync client (`SSHClient`) and async client (`AsyncSSHClient`).
- Run remote commands and get `stdout`, `stderr` and exit code (`rc`).
- Password authentication, plus SSH agent and default key files (`~/.ssh`).
- Host key verification through `known_hosts` (`Strict` by default).
- Context managers (`with` / `async with`) disconnect automatically.
- Typed errors (`SSHError` and subclasses) for connect, auth and command failures.

## Sync example

```python
from ssh2rs import SSHClient

with SSHClient.connect("example.com", 22) as client:
    client.authenticate("user", "password")  # password=None tries agent and ~/.ssh keys first
    result = client.exec("uname -a")
    print(result.stdout, result.stderr, result.rc)
```

## Async example

```python
import asyncio

from ssh2rs import AsyncSSHClient


async def main():
    async with await AsyncSSHClient.connect("example.com", 22) as client:
        await client.authenticate("user", "password")  # password=None tries agent and ~/.ssh keys first
        result = await client.exec("uname -a")
        print(result.stdout, result.stderr, result.rc)


asyncio.run(main())
```

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

Tests need a reachable SSH server set in `.env` (`SSH2RS_TEST_HOST`, `SSH2RS_TEST_PORT`, `SSH2RS_TEST_USERNAME`, optional `SSH2RS_TEST_PASSWORD`).

## Contribution

Please feel free to present your idea by code example (pull request) or reported issues.

**Warning:** before opening an issue or pull request, first create a discussion in the [Discussions](https://github.com/pchomik/ssh2rs/discussions) tab. Describe the idea or problem there first.

- Issues: [github.com/pchomik/ssh2rs/issues](https://github.com/pchomik/ssh2rs/issues)
- Repository: [github.com/pchomik/ssh2rs](https://github.com/pchomik/ssh2rs)

## License

MIT. See [LICENSE](LICENSE).
