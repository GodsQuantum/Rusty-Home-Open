#!/usr/bin/env python3
"""P1e: true SIGKILL -> Rust restart -> durable manual quarantine, synthetic only."""
from __future__ import annotations
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
if os.geteuid() == 0 or not (
    os.uname().nodename == "dev-build" or os.environ.get("GITHUB_ACTIONS") == "true"
):
    raise SystemExit("P1e lab tests allowed only on isolated Linux lab or CI, never root")
subprocess.run(["cargo", "build", "--locked", "--quiet", "-p", "hars-native-kernel"],
               cwd=ROOT, check=True, timeout=240)
binary = ROOT / "target/debug/hars-native-kernel"
(ROOT / "runtime").mkdir(mode=0o700, exist_ok=True)

with tempfile.TemporaryDirectory(prefix="hars-reconcile-", dir=ROOT / "runtime") as location:
    os.chmod(location, 0o700)

    def call(*args: str, deny: bool = False):
        proc = subprocess.run(
            [str(binary), "--lab-ledger", location, *args],
            cwd=ROOT, capture_output=True, text=True, timeout=15,
        )
        if deny:
            assert proc.returncode != 0, f"incorrectly accepted {args}"
            return None
        assert proc.returncode == 0, f"failed {args}: {proc.stderr[-350:]}"
        return json.loads(proc.stdout)

    assert call("prepare", "uncertain-one", "true")["phase"] == "prepared"
    child = subprocess.Popen(
        [str(binary), "--lab-ledger", location, "hold-inflight", "uncertain-one"],
        cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    try:
        assert child.stdout is not None
        event = child.stdout.readline()
        assert event and json.loads(event)["phase"] == "in_flight"
        child.send_signal(signal.SIGKILL)
        assert child.wait(timeout=8) == -signal.SIGKILL
    finally:
        if child.poll() is None:
            child.kill()
            child.wait(timeout=8)
        if child.stdout:
            child.stdout.close()
        if child.stderr:
            child.stderr.close()
    print("PASS: real SIGKILL interrupted an in-flight command, no hardware accessed")

    todo = call("pending")
    assert len(todo) == 1 and todo[0]["id"] == "uncertain-one"
    assert todo[0]["phase"] == "in_flight"
    call("claim", "uncertain-one", deny=True)
    call("quarantine", "uncertain-one", "../unsafe-path", deny=True)
    call("quarantine", "uncertain-one", "", deny=True)
    assert call("review", "uncertain-one") is None
    print("PASS: restarted Rust process lists ambiguous effect, refuses implicit replay and unsafe proof")

    quarantined = call("quarantine", "uncertain-one", "synthetic_observation_19oct")
    assert quarantined["phase"] == "quarantined"
    review = call("review", "uncertain-one")
    assert review == {
        "version": 1, "id": "uncertain-one",
        "evidence_ref": "synthetic_observation_19oct",
        "outcome": "effect_unknown_no_retry",
    }
    after = call("pending")
    assert len(after) == 1 and after[0]["phase"] == "quarantined"
    assert call("status", "uncertain-one")["phase"] == "quarantined"
    call("claim", "uncertain-one", deny=True)
    call("ack", "uncertain-one", deny=True)
    call("reject", "uncertain-one", deny=True)
    call("quarantine", "uncertain-one", "different_proof", deny=True)
    call("prepare", "uncertain-one", "false", deny=True)
    assert call("prepare", "uncertain-one", "true")["phase"] == "quarantined"
    print("PASS: atomic persistent quarantine and evidence, no ACK, no retry or ambiguous success")

    path = Path(location) / "commands.redb"
    assert path.is_file() and not path.is_symlink()
    assert path.stat().st_mode & 0o077 == 0
    assert call("pending")[0]["phase"] == "quarantined"
    print("PASS: separate Rust process reopens owner-private redb, observes identical quarantine")

print("PASS: temporary redb fixture deleted, no service or process retained")
