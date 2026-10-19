"""Async tests for the ssh2rs AsyncSSHClient Python interface.

Run: source .env && pytest -v
Needs SSH2RS_TEST_HOST, SSH2RS_TEST_PORT, SSH2RS_TEST_USERNAME; SSH2RS_TEST_PASSWORD optional.
"""

import asyncio
import contextlib
import os
import socket

import pytest
import pytest_asyncio
import ssh2rs
from ssh2rs import AsyncSSHClient

pytestmark = pytest.mark.asyncio

HOST = os.environ["SSH2RS_TEST_HOST"]
PORT = int(os.environ["SSH2RS_TEST_PORT"])
USERNAME = os.environ["SSH2RS_TEST_USERNAME"]
PASSWORD = os.environ.get("SSH2RS_TEST_PASSWORD") or None


def closed_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


async def connect_authenticated():
    client = await AsyncSSHClient.connect(HOST, PORT)
    await client.authenticate(USERNAME, PASSWORD)
    return client


@pytest_asyncio.fixture
async def client():
    client = await connect_authenticated()
    yield client
    with contextlib.suppress(ssh2rs.NotConnected):
        await client.disconnect()


async def test_exec_stdout(client):
    result = await client.exec("echo hello")
    assert result.stdout == "hello\n"
    assert result.stderr == ""
    assert result.rc == 0


async def test_exec_stderr_and_exit_code(client):
    result = await client.exec("echo err >&2; exit 3")
    assert result.stdout == ""
    assert result.stderr == "err\n"
    assert result.rc == 3


async def test_exec_large_output(client):
    result = await client.exec("seq 1 100000")
    lines = result.stdout.splitlines()
    assert len(lines) == 100000
    assert lines[-1] == "100000"
    assert result.rc == 0


async def test_exec_concurrent(client):
    results = await asyncio.gather(*(client.exec(f"echo {i}") for i in range(5)))
    assert [r.stdout for r in results] == [f"{i}\n" for i in range(5)]


async def test_exec_after_disconnect_raises(client):
    await client.disconnect()
    with pytest.raises(ssh2rs.NotConnected):
        await client.exec("echo hello")


async def test_async_with_disconnects():
    client = await connect_authenticated()
    async with client:
        assert (await client.exec("echo ctx")).stdout == "ctx\n"
    with pytest.raises(ssh2rs.NotConnected):
        await client.exec("echo after")


async def test_connect_refused_raises_connect_error():
    with pytest.raises(ssh2rs.Connect):
        await AsyncSSHClient.connect("127.0.0.1", closed_port())
