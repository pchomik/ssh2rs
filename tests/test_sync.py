"""Sync tests for the ssh2rs SSHClient Python interface.

Run: source .env && pytest -v
Needs SSH2RS_TEST_HOST, SSH2RS_TEST_PORT, SSH2RS_TEST_USERNAME; SSH2RS_TEST_PASSWORD optional.
"""

import contextlib
import os
import socket
from collections.abc import Iterator

import pytest
import ssh2rs
from ssh2rs import HostKeyPolicy, SSHClient

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


@pytest.fixture
def client() -> Iterator[SSHClient]:
    client = SSHClient.connect(host=HOST, port=PORT, timeout=CONNECTION_TIMEOUT, host_key_policy=POLICY)
    client.authenticate(username=USERNAME, password=PASSWORD)
    yield client
    with contextlib.suppress(ssh2rs.NotConnected):
        client.disconnect()


def test_exec_stdout(client: SSHClient) -> None:
    result = client.exec(cmd="echo hello", timeout=EXECUTION_TIMEOUT)
    assert result.stdout == "hello\n"
    assert result.stderr == ""
    assert result.rc == 0


def test_exec_stderr_and_exit_code(client: SSHClient) -> None:
    result = client.exec(cmd="echo err >&2; exit 3", timeout=EXECUTION_TIMEOUT)
    assert result.stdout == ""
    assert result.stderr == "err\n"
    assert result.rc == 3


def test_exec_large_output(client: SSHClient) -> None:
    result = client.exec(cmd="seq 1 100000", timeout=EXECUTION_TIMEOUT)
    lines = result.stdout.splitlines()
    assert len(lines) == 100000
    assert lines[-1] == "100000"
    assert result.rc == 0


def test_exec_multiple_commands(client: SSHClient) -> None:
    for i in range(5):
        assert client.exec(cmd=f"echo {i}", timeout=EXECUTION_TIMEOUT).stdout == f"{i}\n"


def test_exec_after_disconnect_raises(client: SSHClient) -> None:
    client.disconnect()
    with pytest.raises(ssh2rs.NotConnected):
        client.exec(cmd="echo hello", timeout=EXECUTION_TIMEOUT)


def test_with_block_disconnects() -> None:
    with SSHClient.connect(host=HOST, port=PORT, timeout=CONNECTION_TIMEOUT, host_key_policy=POLICY) as client:
        client.authenticate(username=USERNAME, password=PASSWORD)
        assert client.exec(cmd="echo ctx", timeout=EXECUTION_TIMEOUT).stdout == "ctx\n"
    with pytest.raises(ssh2rs.NotConnected):
        client.exec(cmd="echo after", timeout=EXECUTION_TIMEOUT)


def test_connect_refused_raises_connect_error(closed_port: int) -> None:
    with pytest.raises(ssh2rs.Connect):
        SSHClient.connect(host="127.0.0.1", port=closed_port, timeout=CONNECTION_TIMEOUT)


def test_exec_timeout_raises_timeout(client: SSHClient) -> None:
    with pytest.raises(ssh2rs.Timeout):
        client.exec(cmd="sleep 10", timeout=0.5)


def test_connect_timeout_raises_timeout(silent_port: int) -> None:
    with pytest.raises(ssh2rs.Timeout):
        SSHClient.connect(host="127.0.0.1", port=silent_port, timeout=0.5)
