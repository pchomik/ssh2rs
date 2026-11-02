"""Sync tests for the ssh2rs SSHClient Python interface.

Run: source .env && pytest -v
Needs SSH2RS_TEST_HOST, SSH2RS_TEST_PORT, SSH2RS_TEST_USERNAME; SSH2RS_TEST_PASSWORD optional.
"""

import contextlib
import os
import socket
import uuid
from collections.abc import Iterator
from pathlib import Path

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


def test_latest_result_property_returns_last_exec_output(client: SSHClient) -> None:
    client.exec(cmd="echo first", timeout=EXECUTION_TIMEOUT)
    client.exec(cmd="echo out; echo err >&2", timeout=EXECUTION_TIMEOUT)
    latest = client.latest_result
    assert latest.stdout == "out\n"
    assert latest.stderr == "err\n"


def test_latest_result_property_empty_before_exec(client: SSHClient) -> None:
    assert client.latest_result.stdout == ""
    assert client.latest_result.stderr == ""


@pytest.fixture
def remote_path(client: SSHClient) -> Iterator[str]:
    path = f"/tmp/ssh2rs-test-{uuid.uuid4().hex}"
    yield path
    with contextlib.suppress(ssh2rs.NotConnected):
        client.exec(cmd=f"rm -f {path}", timeout=EXECUTION_TIMEOUT)


def test_upload_writes_remote_file(client: SSHClient, tmp_path: Path, remote_path: str) -> None:
    src = tmp_path / "src.txt"
    src.write_text("hello upload\n")
    client.upload(src=str(src), dst=remote_path, timeout=EXECUTION_TIMEOUT)
    assert client.exec(cmd=f"cat {remote_path}", timeout=EXECUTION_TIMEOUT).stdout == "hello upload\n"


def test_upload_overwrites_existing_remote_file(client: SSHClient, tmp_path: Path, remote_path: str) -> None:
    client.exec(cmd=f"head -c 1000 /dev/zero > {remote_path}", timeout=EXECUTION_TIMEOUT)
    src = tmp_path / "src.txt"
    src.write_text("new\n")
    client.upload(src=str(src), dst=remote_path, timeout=EXECUTION_TIMEOUT)
    assert client.exec(cmd=f"cat {remote_path}", timeout=EXECUTION_TIMEOUT).stdout == "new\n"


def test_upload_then_download_round_trip_binary(client: SSHClient, tmp_path: Path, remote_path: str) -> None:
    content = bytes(range(256)) * 4096  # 1 MiB, all byte values
    src = tmp_path / "src.bin"
    src.write_bytes(content)
    back = tmp_path / "back.bin"
    client.upload(src=str(src), dst=remote_path, timeout=EXECUTION_TIMEOUT)
    client.download(src=remote_path, dst=str(back), timeout=EXECUTION_TIMEOUT)
    assert back.read_bytes() == content


def test_download_writes_local_file(client: SSHClient, tmp_path: Path, remote_path: str) -> None:
    client.exec(cmd=f"printf 'remote data\\n' > {remote_path}", timeout=EXECUTION_TIMEOUT)
    dst = tmp_path / "dst.txt"
    client.download(src=remote_path, dst=str(dst), timeout=EXECUTION_TIMEOUT)
    assert dst.read_text() == "remote data\n"


def test_upload_missing_local_file_raises_file_error(client: SSHClient, tmp_path: Path, remote_path: str) -> None:
    with pytest.raises(ssh2rs.SSHError):
        client.upload(src=str(tmp_path / "missing.txt"), dst=remote_path, timeout=EXECUTION_TIMEOUT)


def test_download_missing_remote_file_raises_sftp(client: SSHClient, tmp_path: Path, remote_path: str) -> None:
    with pytest.raises(ssh2rs.SSHError):
        client.download(src=remote_path, dst=str(tmp_path / "dst.txt"), timeout=EXECUTION_TIMEOUT)


def test_upload_timeout_raises_timeout(client: SSHClient, tmp_path: Path, remote_path: str) -> None:
    src = tmp_path / "src.txt"
    src.write_text("data\n")
    with pytest.raises(ssh2rs.Timeout):
        client.upload(src=str(src), dst=remote_path, timeout=1e-6)


def test_download_timeout_raises_timeout(client: SSHClient, tmp_path: Path, remote_path: str) -> None:
    client.exec(cmd=f"echo data > {remote_path}", timeout=EXECUTION_TIMEOUT)
    with pytest.raises(ssh2rs.Timeout):
        client.download(src=remote_path, dst=str(tmp_path / "dst.txt"), timeout=1e-6)


def test_upload_after_disconnect_raises(client: SSHClient, tmp_path: Path) -> None:
    src = tmp_path / "src.txt"
    src.write_text("data\n")
    client.disconnect()
    with pytest.raises(ssh2rs.NotConnected):
        client.upload(src=str(src), dst="/tmp/ssh2rs-unused", timeout=EXECUTION_TIMEOUT)


def test_download_after_disconnect_raises(client: SSHClient, tmp_path: Path) -> None:
    client.disconnect()
    with pytest.raises(ssh2rs.NotConnected):
        client.download(src="/tmp/ssh2rs-unused", dst=str(tmp_path / "dst.txt"), timeout=EXECUTION_TIMEOUT)
