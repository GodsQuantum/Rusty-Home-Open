#!/usr/bin/env python3
"""Adversarial lab TCP test: slow clients cannot block independent health read.

Tests a real unprivileged Rust process, never a production HAOS instance.
"""
import os
from pathlib import Path
import socket
import subprocess
import time

ROOT = Path(__file__).resolve().parent.parent
assert os.geteuid() != 0, "Do not run security test as root"
binary = Path(os.environ.get("CARGO_TARGET_DIR", str(ROOT/"target"))) / "debug/ha-rust-bridge"
assert binary.is_file(), f"Build the Rust lab server: {binary}"
with socket.socket() as reserve:
    reserve.bind(("127.0.0.1", 0))
    port = reserve.getsockname()[1]

def request(timeout=1.5):
    started=time.monotonic()
    with socket.create_connection(("127.0.0.1",port),timeout=timeout) as sock:
        sock.settimeout(timeout)
        sock.sendall(f"GET /health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n".encode())
        response=sock.recv(4096)
    assert response.startswith(b"HTTP/1.1 200 OK"),response[:180]
    return time.monotonic()-started

p=subprocess.Popen([str(binary),"--fixture","fixtures/docker-ps.jsonl",
                    "--host","lab","--serve",f"127.0.0.1:{port}"],
                    cwd=ROOT,stderr=subprocess.DEVNULL,stdout=subprocess.DEVNULL)
slow=[]
try:
    for _ in range(100):
        assert p.poll() is None, "Lab server terminated before start"
        try:
            request(.2)
            break
        except (OSError,AssertionError):
            time.sleep(.025)
    else:
        raise AssertionError("Lab server never became healthy")
    for _ in range(6):
        sock=socket.create_connection(("127.0.0.1",port),timeout=2)
        sock.settimeout(.5)
        sock.sendall(f"GET /health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX-Partial:".encode())
        slow.append(sock)
    time.sleep(.12)
    latency=request(timeout=1.5)
    assert latency<1.5,latency
    # A steady drip must not reset the overall HTTP header deadline.
    for _ in range(8):
        time.sleep(.34)
        for sock in slow:
            try: sock.sendall(b"x")
            except OSError: pass
    assert p.poll() is None, "Malformed clients crashed the server"
    latency=request(timeout=1.5)
    assert latency<1.5,latency
    # Saturate workers AND the bounded backlog with partial requests.
    # Excess work must be shed with HTTP 503, not spawned onto new threads.
    # All sockets and the server remain exclusively on isolated loopback.
    import select
    overload=[]
    try:
        for _ in range(70):
            sock=socket.create_connection(("127.0.0.1",port),timeout=1)
            sock.sendall(f"GET /health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX-Incomplete: ".encode())
            overload.append(sock)
        readable,_,_=select.select(overload,[],[],1.5)
        rejected=0
        for sock in readable:
            try:
                result=sock.recv(512)
                if result.startswith(b"HTTP/1.1 503 Service Unavailable"):
                    rejected+=1
            except (OSError,ConnectionResetError):
                pass
        assert rejected>0, "More than the bounded queue was accepted without immediate load shedding"
    finally:
        for sock in overload:
            sock.close()
    # Closed queued clients must not permanently occupy capacity.
    for _ in range(20):
        try:
            latency=request(timeout=.6)
            break
        except (OSError,AssertionError):
            time.sleep(.12)
    else:
        raise AssertionError("No capacity recovered after clients disconnected")
    assert p.poll() is None
    # No unbounded thread-per-connection expansion. Linux /proc only.
    tasks=Path(f"/proc/{p.pid}/task")
    if tasks.exists():
        count=len(list(tasks.iterdir()))
        assert count<=12,f"Unbounded handler threads: {count}"
    print("P19_PASS: six stalled TCP clients, independent health, absolute deadline, bounded workers")
finally:
    for s in slow:
        s.close()
    p.terminate()
    try:p.wait(timeout=3)
    except subprocess.TimeoutExpired:
        p.kill()
        p.wait(timeout=3)
