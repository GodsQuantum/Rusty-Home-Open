#!/usr/bin/env python3
"""Offline Rust+Python IPC integration: client, full app, HTTP, deny token."""
import http.client
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent
assert os.geteuid() != 0, "No root tests"
(ROOT / "runtime").mkdir(mode=0o700, exist_ok=True)
subprocess.run(["cargo", "build", "--locked", "-p", "hars-python-bridge",
                "-p", "ha-rust-bridge", "--quiet"], cwd=ROOT, check=True, timeout=240)
client_bin = ROOT / "target/debug/hars-python-bridge"
app_bin = ROOT / "target/debug/ha-rust-bridge"

for mode in ("client", "app", "http", "reject"):
    with tempfile.TemporaryDirectory(prefix="hars-ipc-", dir=ROOT / "runtime") as folder:
        directory = Path(folder)
        os.chmod(directory, 0o700)
        endpoint, cap = directory / "mock.sock", directory / "capability"
        cap.write_text("A" * 48, encoding="ascii")
        os.chmod(cap, 0o600)
        server = subprocess.Popen(
            [sys.executable, str(ROOT / "scripts/python_sidecar_fixture.py"),
             str(endpoint), str(cap)], cwd=ROOT,
            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        app = None
        try:
            for _ in range(200):
                if endpoint.exists() or server.poll() is not None:
                    break
                time.sleep(0.025)
            assert endpoint.exists(), "Fixture sidecar did not start"
            if mode == "reject":
                cap.write_text("B" * 48, encoding="ascii")
                os.chmod(cap, 0o600)
            args = [str(client_bin), str(endpoint), str(cap)]
            if mode in ("app", "http"):
                args = [str(app_bin), "--fixture", str(ROOT / "fixtures/docker-ps.jsonl"),
                        "--host", "lab-node", "--python-socket", str(endpoint),
                        "--python-capability", str(cap)]
            if mode == "http":
                with socket.socket() as reserve:
                    reserve.bind(("127.0.0.1", 0))
                    port = reserve.getsockname()[1]
                app = subprocess.Popen(args + ["--serve", f"127.0.0.1:{port}"],
                                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                for _ in range(200):
                    try:
                        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=1)
                        connection.request("GET", "/health")
                        response = connection.getresponse()
                        status = response.status
                        response.read()
                        connection.close()
                        if status == 200:
                            break
                    except OSError:
                        time.sleep(.025)
                else:
                    raise AssertionError("Rust HTTP test service did not start")
                connection = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
                connection.request("GET", "/api/python-core")
                response = connection.getresponse()
                data = json.loads(response.read())
                connection.close()
                assert response.status == 200 and data["entities_count"] == 1232
                assert data["contains_entity_values"] is False
                assert data["production_ready"] is False
            else:
                result = subprocess.run(args, capture_output=True, text=True, timeout=8)
                assert (result.returncode != 0) == (mode == "reject")
                if mode == "client":
                    assert "states=1232" in result.stdout
                if mode == "app":
                    data = json.loads(result.stdout)
                    assert data["python_core"]["entities_count"] == 1232
                    assert data["python_core"]["contains_entity_values"] is False
                    assert len(data["apps"]) == 4
            server.wait(timeout=8)
            assert server.returncode == 0
        finally:
            if app is not None and app.poll() is None:
                app.terminate()
                app.wait(timeout=5)
            if server.poll() is None:
                server.kill()
                server.wait(timeout=5)
            if server.stderr:
                server.stderr.close()
    print(f"PASS: Rust-Python IPC scenario {mode}")
print("PASS: no persistent socket or lab HTTP listener")
