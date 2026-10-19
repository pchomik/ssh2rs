"""Sync tests for the ssh2rs SSHClient Python interface.

Run: source .env && pytest -v
Needs SSH2RS_TEST_HOST, SSH2RS_TEST_PORT, SSH2RS_TEST_USERNAME; SSH2RS_TEST_PASSWORD optional.
"""

import contextlib
import os
import socket

import pytest
import ssh2rs
from ssh2rs import SSHClient

HOST = os.environ["SSH2RS_TEST_HOST"]
PORT = int(os.environ["SSH2RS_TEST_PORT"])
USERNAME = os.environ["SSH2RS_TEST_USERNAME"]
PASSWORD = os.environ.get("SSH2RS_TEST_PASSWORD") or None


def closed_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


@pytest.fixture
def client():
    client = SSHClient.connect(HOST, PORT)
    client.authenticate(USERNAME, PASSWORD)
    yield client
    with contextlib.suppress(ssh2rs.NotConnected):
        client.disconnect()


def test_exec_stdout(client):
    result = client.exec("echo hello")
    assert result.stdout == "hello\n"
    assert result.stderr == ""
    assert result.rc == 0


def test_exec_stderr_and_exit_code(client):
    result = client.exec("echo err >&2; exit 3")
    assert result.stdout == ""
    assert result.stderr == "err\n"
    assert result.rc == 3


def test_exec_large_output(client):
    result = client.exec("seq 1 100000")
    lines = result.stdout.splitlines()
    assert len(lines) == 100000
    assert lines[-1] == "100000"
    assert result.rc == 0


def test_exec_multiple_commands(client):
    for i in range(5):
        assert client.exec(f"echo {i}").stdout == f"{i}\n"


def test_exec_after_disconnect_raises(client):
    client.disconnect()
    with pytest.raises(ssh2rs.NotConnected):
        client.exec("echo hello")


def test_with_block_disconnects():
    with SSHClient.connect(HOST, PORT) as client:
        client.authenticate(USERNAME, PASSWORD)
        assert client.exec("echo ctx").stdout == "ctx\n"
    with pytest.raises(ssh2rs.NotConnected):
        client.exec("echo after")


def test_connect_refused_raises_connect_error():
    with pytest.raises(ssh2rs.Connect):
        SSHClient.connect("127.0.0.1", closed_port())
