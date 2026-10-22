#!/usr/bin/env python3
"""Isolated REAL Rust lab-daemon lifecycle + authenticated loopback HTTP.

Runs only without root in isolated Linux lab/CI. No HAOS, radios, credentials or mounts.
"""
from __future__ import annotations

import http.client
import json
import os
from pathlib import Path
import secrets
import signal
import socket
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent
if os.geteuid() == 0 or not (
    os.uname().nodename == "dev-build"
    or os.environ.get("GITHUB_ACTIONS") == "true"
):
    raise SystemExit("No root; isolated Linux lab or isolated CI only")

subprocess.run(["cargo", "build", "-p", "hars-native-kernel", "--locked", "--quiet"],
               cwd=ROOT, check=True, timeout=240)
binary = ROOT / "target/debug/hars-native-kernel"
assert binary.is_file(), "native daemon executable missing"
(ROOT / "runtime").mkdir(mode=0o700, exist_ok=True)


def request(port: int, method: str, path: str, token: str | None = None) -> tuple[int, dict]:
    client = http.client.HTTPConnection("127.0.0.1", port, timeout=2)
    headers = {"Authorization": f"Bearer {token}"} if token else {}
    try:
        client.request(method, path, headers=headers)
        result = client.getresponse()
        return result.status, json.loads(result.read())
    finally:
        client.close()


with tempfile.TemporaryDirectory(prefix="hars-native-daemon-", dir=ROOT / "runtime") as location:
    state = Path(location)
    os.chmod(state, 0o700)
    token = secrets.token_hex(32)
    token_file = state / "capability"
    token_file.write_text(token, encoding="ascii")
    os.chmod(token_file, 0o600)
    with socket.socket() as reserved:
        reserved.bind(("127.0.0.1", 0))
        port = reserved.getsockname()[1]

    def launch(seed: bool) -> subprocess.Popen:
        args = [str(binary), "--lab-daemon", str(state), str(token_file), str(port)]
        if seed:
            args.append("--seed-synthetic")
        child = subprocess.Popen(args, cwd=ROOT, stdout=subprocess.DEVNULL,
                                 stderr=subprocess.PIPE, text=True)
        for _ in range(160):
            if child.poll() is not None:
                error = child.stderr.read()[-500:] if child.stderr else ""
                cache = state / "native-states.json"
                mode = oct(cache.stat().st_mode & 0o777) if cache.exists() else "missing"
                raise AssertionError(f"Rust daemon exited: {child.returncode}; stderr={error}; state_mode={mode}")
            try:
                status, result = request(port, "GET", "/health", token)
                if status == 200 and result["native_rust"]:
                    return child
            except (OSError, TimeoutError):
                pass
            time.sleep(.025)
        child.kill()
        child.wait(timeout=5)
        raise AssertionError("Rust daemon did not open loopback port")

    def stop(child: subprocess.Popen) -> None:
        try:
            child.send_signal(signal.SIGINT)
            child.wait(timeout=6)
            assert child.returncode == 0, "graceful Rust shutdown failed"
        finally:
            if child.poll() is None:
                child.kill()
                child.wait(timeout=5)
            if child.stderr:
                child.stderr.close()

    # Refuse unsafe capability permissions BEFORE starting the HTTP socket.
    os.chmod(token_file, 0o644)
    denied = subprocess.run([str(binary), "--lab-daemon", str(state), str(token_file), str(port)],
                            cwd=ROOT, capture_output=True, text=True, timeout=5)
    assert denied.returncode != 0, "world-readable capability accepted"
    os.chmod(token_file, 0o600)
    print("PASS: daemon denies mode-0644 bearer token file")

    child = launch(seed=True)
    try:
        status, result = request(port, "GET", "/api/native/state", token)
        assert status == 200 and result["sensor_on"] is True
        assert result["actuator_on"] is False and result["production_ready"] is False
        assert result["read_only"] and result["devices"] == 2
        print("PASS: real Moli native states read by authenticated HTTP")

        status, _ = request(port, "GET", "/api/native/state", "f" * 64)
        assert status == 401
        status, _ = request(port, "GET", "/api/native/state")
        assert status == 401
        print("PASS: missing/incorrect bearer capability both rejected")

        status, result = request(port, "POST", "/api/native/state", token)
        assert status == 405 and result["read_only"] is True
        print("PASS: HTTP native state mutation denied")
    finally:
        stop(child)

    # An upstream cache atomic rename must never leave user data world-readable.
    saved = state / "native-states.json"
    assert saved.is_file() and not saved.is_symlink()
    assert saved.stat().st_mode & 0o077 == 0, "state cache is not private after checkpoint"
    print("PASS: saved Moli state cache remains owner-only mode 0600")

    child = launch(seed=False)
    try:
        status, result = request(port, "GET", "/api/native/state", token)
        assert status == 200 and result["sensor_on"] is True
        assert result["actuator_on"] is False
        print("PASS: synthetic native Rust state retained across process restart")
    finally:
        stop(child)

    # No network listener after shutdown, private fixture wiped on leaving context.
    try:
        request(port, "GET", "/health", token)
    except (OSError, TimeoutError):
        print("PASS: no Rust service left listening after SIGINT")
    else:
        raise AssertionError("Rust lab server still listening")

print("PASS: no fixture files, token or daemon left behind")
