#!/usr/bin/env python3
"""P1d synthetic redb command reservation test with a real SIGKILL process boundary.
The Rust binary does not depend on Python. No hardware is dispatched.
"""
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
if os.geteuid() == 0 or not (os.uname().nodename == "dev-build" or
                            os.environ.get("GITHUB_ACTIONS") == "true"):
    raise SystemExit("P1d tests must run unprivileged on isolated Linux lab or CI")
subprocess.run(["cargo", "build", "--quiet", "--locked", "-p", "hars-native-kernel"],
               cwd=ROOT, timeout=240, check=True)
binary = ROOT / "target/debug/hars-native-kernel"
assert binary.is_file()
(ROOT / "runtime").mkdir(mode=0o700, exist_ok=True)

with tempfile.TemporaryDirectory(prefix="hars-ledger-kill9-", dir=ROOT / "runtime") as directory:
    os.chmod(directory, 0o700)
    def call(*args, fail=False):
        p=subprocess.run([str(binary), "--lab-ledger", directory, *args], cwd=ROOT,
                         text=True, capture_output=True, timeout=12)
        if fail:
            assert p.returncode != 0, f"should refuse {args}: {p.stdout}"
            return None
        assert p.returncode == 0, f"{args}: {p.stderr[-450:]}"
        return json.loads(p.stdout.strip())

    first=call("prepare","kill-case-1","true")
    assert first["phase"]=="prepared" and first["target"]=="hars-lab:actuator/on"
    assert call("prepare","kill-case-1","true")==first
    call("prepare","kill-case-1","false",fail=True)
    print("PASS: prepare transaction durably deduplicated, conflicting key refused")

    # The process has already committed InFlight before printing. Kill before ACK.
    child=subprocess.Popen([str(binary),"--lab-ledger",directory,
                            "hold-inflight","kill-case-1"],cwd=ROOT,
                           text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    try:
        line=child.stdout.readline()
        assert line and json.loads(line)["phase"]=="in_flight", "missing durable claim"
        child.send_signal(signal.SIGKILL)
        assert child.wait(timeout=8)==-signal.SIGKILL
    finally:
        if child.poll() is None:
            child.kill()
            child.wait(timeout=8)
        child.stdout.close()
        child.stderr.close()
    after=call("status","kill-case-1")
    assert after["phase"]=="in_flight"
    assert call("prepare","kill-case-1","true")["phase"]=="in_flight"
    call("claim","kill-case-1",fail=True)
    call("prepare","kill-case-1","false",fail=True)
    print("PASS: process SIGKILL leaves durable IN_FLIGHT with NO implicit redelivery")

    # A different command can be explicitly acknowledged, and stays so after reopen.
    assert call("prepare","normal-case-2","false")["phase"]=="prepared"
    assert call("claim","normal-case-2")["phase"]=="in_flight"
    assert call("ack","normal-case-2")["phase"]=="acknowledged"
    assert call("status","normal-case-2")["phase"]=="acknowledged"
    call("claim","normal-case-2",fail=True)
    call("ack","normal-case-2",fail=True)
    print("PASS: only explicit acknowledged transitions committed exactly once in DB")

    db=Path(directory)/"commands.redb"
    assert db.is_file() and not db.is_symlink()
    assert db.stat().st_mode&0o077==0, "transactional DB not owner-private"
    print("PASS: owner-private redb file, all fixture data isolated")

print("PASS: fixture database removed after tests, no background processes")
