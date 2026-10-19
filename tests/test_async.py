"""Async tests for the ssh2rs AsyncSSHClient Python interface.

Run: source .env && pytest -v
Needs SSH2RS_TEST_HOST, SSH2RS_TEST_PORT, SSH2RS_TEST_USERNAME; SSH2RS_TEST_PASSWORD optional.
"""

import asyncio
import contextlib
import os
import socket
from collections.abc import AsyncIterator, Iterator

import pytest
import pytest_asyncio
import ssh2rs
from ssh2rs import AsyncSSHClient, HostKeyPolicy

pytestmark = pytest.mark.asyncio

HOST = os.environ["SSH2RS_TEST_HOST"]
PORT = int(os.environ["SSH2RS_TEST_PORT"])
USERNAME = os.environ["SSH2RS_TEST_USERNAME"]
PASSWORD = os.environ.get("SSH2RS_TEST_PASSWORD") or None
POLICY = {"disabled": HostKeyPolicy.Disabled, "accept_new": HostKeyPolicy.AcceptNew}.get(
    os.environ.get("SSH2RS_TEST_POLICY", ""), HostKeyPolicy.Strict
)
CONNECTION_TIMEOUT = 5.0
EXECUTION_TIMEOUT = 2.0


@pytest.fixture
def closed_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


@pytest.fixture
def silent_port() -> Iterator[int]:
    # Accepts TCP but never speaks SSH, so the handshake hangs.
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        s.listen()
        yield s.getsockname()[1]


async def connect_authenticated() -> AsyncSSHClient:
    client = await AsyncSSHClient.connect(host=HOST, port=PORT, timeout=CONNECTION_TIMEOUT, host_key_policy=POLICY)
    await client.authenticate(username=USERNAME, password=PASSWORD)
    return client


@pytest_asyncio.fixture
async def client() -> AsyncIterator[AsyncSSHClient]:
    client = await connect_authenticated()
    yield client
    with contextlib.suppress(ssh2rs.NotConnected):
        await client.disconnect()


async def test_exec_stdout(client: AsyncSSHClient) -> None:
    result = await client.exec(cmd="echo hello", timeout=EXECUTION_TIMEOUT)
    assert result.stdout == "hello\n"
    assert result.stderr == ""
    assert result.rc == 0


async def test_exec_stderr_and_exit_code(client: AsyncSSHClient) -> None:
    result = await client.exec(cmd="echo err >&2; exit 3", timeout=EXECUTION_TIMEOUT)
    assert result.stdout == ""
    assert result.stderr == "err\n"
    assert result.rc == 3


async def test_exec_large_output(client: AsyncSSHClient) -> None:
    result = await client.exec(cmd="seq 1 100000", timeout=EXECUTION_TIMEOUT)
    lines = result.stdout.splitlines()
    assert len(lines) == 100000
    assert lines[-1] == "100000"
    assert result.rc == 0


async def test_exec_concurrent(client: AsyncSSHClient) -> None:
    results = await asyncio.gather(*(client.exec(cmd=f"echo {i}", timeout=EXECUTION_TIMEOUT) for i in range(5)))
    assert [r.stdout for r in results] == [f"{i}\n" for i in range(5)]


async def test_exec_after_disconnect_raises(client: AsyncSSHClient) -> None:
    await client.disconnect()
    with pytest.raises(ssh2rs.NotConnected):
        await client.exec(cmd="echo hello", timeout=EXECUTION_TIMEOUT)


async def test_async_with_disconnects() -> None:
    client = await connect_authenticated()
    async with client:
        assert (await client.exec(cmd="echo ctx", timeout=EXECUTION_TIMEOUT)).stdout == "ctx\n"
    with pytest.raises(ssh2rs.NotConnected):
        await client.exec(cmd="echo after", timeout=EXECUTION_TIMEOUT)


async def test_connect_refused_raises_connect_error(closed_port: int) -> None:
    with pytest.raises(ssh2rs.Connect):
        await AsyncSSHClient.connect(host="127.0.0.1", port=closed_port, timeout=CONNECTION_TIMEOUT)


async def test_exec_timeout_raises_timeout(client: AsyncSSHClient) -> None:
    with pytest.raises(ssh2rs.Timeout):
        await client.exec(cmd="sleep 10", timeout=0.5)


async def test_connect_timeout_raises_timeout(silent_port: int) -> None:
    with pytest.raises(ssh2rs.Timeout):
        await AsyncSSHClient.connect(host="127.0.0.1", port=silent_port, timeout=0.5)
