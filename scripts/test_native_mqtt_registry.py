#!/usr/bin/env python3
"""One-shot P2g proof: kill a real Rust writer AFTER durable commit, then
restart a distinct Rust process and verify stale-only registry recovery.
No MQTT broker, Docker, household devices or real credentials involved.
"""
from __future__ import annotations
import json
import os
from pathlib import Path
import select
import signal
import stat
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
if os.geteuid() == 0 or not (
    os.uname().nodename == "dev-build"
    or os.environ.get("GITHUB_ACTIONS") == "true"
):
    raise SystemExit("P2g test requires unprivileged isolated Linux lab or GitHub CI")

subprocess.run(
    ["cargo", "build", "--locked", "--quiet", "-p", "hars-native-kernel"],
    cwd=ROOT, check=True, timeout=240
)
binpath = ROOT / "target/debug/hars-native-kernel"
runtime = ROOT / "runtime"
runtime.mkdir(mode=0o700, exist_ok=True)

with tempfile.TemporaryDirectory(prefix="hars-p2g-", dir=runtime) as path:
    testdir = Path(path)
    os.chmod(testdir, 0o700)
    p = subprocess.Popen(
        [str(binpath), "--lab-mqtt-registry-stage", str(testdir)],
        cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True
    )
    try:
        readable, _, _ = select.select([p.stdout], [], [], 10)
        assert readable, "Rust did not durably stage the sensor config"
        assert p.stdout.readline().strip() == "LAB_MQTT_REGISTRY_COMMITTED"
        assert p.poll() is None, "writer exited before true SIGKILL"
        os.kill(p.pid, signal.SIGKILL)
        _, error = p.communicate(timeout=5)
        assert p.returncode == -signal.SIGKILL, f"writer not killed: {error[:200]}"
        db = testdir / "mqtt-discovery.redb"
        assert db.is_file() and stat.S_IMODE(db.stat().st_mode) == 0o600
        recovered = subprocess.run(
            [str(binpath), "--lab-mqtt-registry-recover", str(testdir)],
            cwd=ROOT, capture_output=True, text=True, timeout=10
        )
        assert recovered.returncode == 0, recovered.stderr[:450]
        data = json.loads(recovered.stdout)
        assert data == {
            "registry_entries": 1,
            "stale_unverified": 1,
            "device_ownership_granted": False,
            "production_ready": False,
        }, data
        print("PASS: real SIGKILL after redb durable stage; new process recovers stale-only, zero device authority")

        os.chmod(db, 0o644)
        rejected = subprocess.run(
            [str(binpath), "--lab-mqtt-registry-recover", str(testdir)],
            cwd=ROOT, capture_output=True, text=True, timeout=10
        )
        assert rejected.returncode != 0 and "unsafe" in rejected.stderr.lower()
        print("PASS: independent process refuses permissive registry file")
    finally:
        if p.poll() is None:
            p.kill()
            p.wait(timeout=5)
        if p.stdout:
            p.stdout.close()
        if p.stderr:
            p.stderr.close()

print("PASS: synthetic registry files removed by isolated temporary directory")
