#!/usr/bin/env python3
"""Real raw-socket adversarial test of the Rusty Home LAB HTTP listener."""
import os
import socket
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
target = Path(os.environ.get("CARGO_TARGET_DIR", str(ROOT / "target")))
binary = target / "debug" / "ha-rust-bridge"
assert binary.is_file(), "build the Rust control-plane first"
assert os.getuid() != 0, "lab daemon must run unprivileged"
with socket.socket() as reserve:
    reserve.bind(("127.0.0.1", 0))
    port = reserve.getsockname()[1]


def raw_request(data: bytes, chunks: int = 1) -> tuple[int, bytes]:
    with socket.create_connection(("127.0.0.1", port), timeout=3) as sock:
        sock.settimeout(3)
        if chunks > 1:
            middle = len(data) // chunks
            sock.sendall(data[:middle])
            time.sleep(0.02)
            sock.sendall(data[middle:])
        else:
            sock.sendall(data)
        chunks_recv = []
        while True:
            b = sock.recv(8192)
            if not b:
                break
            chunks_recv.append(b)
    response = b"".join(chunks_recv)
    assert response.startswith(b"HTTP/1.1 "), response[:100]
    status = int(response.split(b" ", 2)[1])
    return status, response


def req(path="/health", host=None, extra="", method="GET") -> bytes:
    if host is None:
        host = f"127.0.0.1:{port}"
    return (f"{method} {path} HTTP/1.1\r\nHost: {host}\r\n{extra}\r\n").encode("ascii")


process = subprocess.Popen(
    [str(binary), "--fixture", "fixtures/docker-ps.jsonl",
     "--host", "lab", "--serve", f"127.0.0.1:{port}"],
    cwd=ROOT,
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
)
try:
    for _ in range(80):
        if process.poll() is not None:
            raise RuntimeError("control-plane exited unexpectedly")
        try:
            status, body = raw_request(req())
            if status == 200:
                break
        except (OSError, AssertionError):
            time.sleep(0.05)
    else:
        raise RuntimeError("Rust lab HTTP not ready")

    valid = [
        req("/api/apps"),
        req("/api/readiness", f"localhost:{port}"),
        req("/health", extra=f"Origin: http://127.0.0.1:{port}\r\n"),
        req("/health", extra="Sec-Fetch-Site: same-origin\r\n"),
        req("/health", extra="Content-Length: 0\r\n"),
    ]
    for data in valid:
        assert raw_request(data)[0] == 200, data[:70]
    assert raw_request(req("/health"), chunks=3)[0] == 200
    assert raw_request(req("/api/apps", method="POST"))[0] == 404

    rejected = [
        req(host=f"evil.example:{port}"),
        req(host="localhost"),
        req(extra=f"Host: 127.0.0.1:{port}\r\n"),
        req(extra="Origin: https://attacker.invalid\r\n"),
        req(extra="Origin: null\r\n"),
        req(extra="Sec-Fetch-Site: cross-site\r\n"),
        req(path=f"http://127.0.0.1:{port}/health"),
        req(extra="Transfer-Encoding: chunked\r\n"),
        req(extra="Content-Length: 999\r\n"),
        req(extra=" Folded: denied\r\n"),
        req(extra="X-Injected: yes\nX-Bad: no\r\n"),
        b"GET /health HTTP/1.1\r\n\r\n",
    ]
    for data in rejected:
        status, response = raw_request(data)
        assert status == 400, (status, data[:100], response[:100])
        assert b"Cache-Control: no-store" in response
    big = req(extra="X-Long: " + "A" * 4300 + "\r\n")
    assert raw_request(big)[0] == 431
    assert raw_request(req("/health"))[0] == 200, "invalid requests must not kill the server"
    print("PASS P16: 20+ genuine TCP tests; Host, Origin, fetch metadata, framing, size, fragmented reads, fail-closed")
finally:
    process.terminate()
    try:
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=3)
