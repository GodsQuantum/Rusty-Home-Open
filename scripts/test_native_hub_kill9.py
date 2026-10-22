#!/usr/bin/env python3
"""SIGKILL only a child process started by this test, NEVER system services.

Disjoint synthetic scratch dirs live under Git-ignored runtime/. Does not run
any home-automation drivers except one in-process fake Moli driver.
"""
from __future__ import annotations
import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent
if os.getuid() == 0 or (
    os.uname().nodename != "dev-build"
    and os.environ.get("GITHUB_ACTIONS") != "true"
):
    raise SystemExit("Run unprivileged in sample lab host isolated Linux lab or isolated GitHub CI only")
if not (ROOT / "crates/native-hub/examples/crash_probe.rs").is_file():
    raise SystemExit("Unexpected working folder")
(ROOT / "runtime").mkdir(mode=0o700,exist_ok=True)

# Build once; test subprocess is a *known child*, not shell/PID lookup.
env = os.environ.copy()
env["CARGO_TARGET_DIR"] = str(ROOT / "runtime/crash-target")
env["CARGO_BUILD_JOBS"] = "4"
subprocess.run(
    ["cargo", "build", "--locked", "--quiet", "-p", "hars-native-hub",
     "--example", "crash_probe"],
    cwd=ROOT, env=env, check=True, timeout=240,
)
binary = ROOT / "runtime/crash-target/debug/examples/crash_probe"
if not binary.is_file():
    raise SystemExit("Rust fixture binary missing")

out = []
with tempfile.TemporaryDirectory(prefix="hars-sigkill-", dir=ROOT / "runtime") as scratch:
    for mode, expected in (("flushed", 32), ("unflushed", 0)):
        path = Path(scratch) / mode / "crash-probe-state.json"
        path.parent.mkdir(mode=0o700)
        proc = subprocess.Popen(
            [str(binary), mode, str(path)],
            cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, bufsize=1,
        )
        try:
            selector = selectors.DefaultSelector()
            assert proc.stdout is not None
            selector.register(proc.stdout, selectors.EVENT_READ)
            ready = ""
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                if proc.poll() is not None:
                    raise RuntimeError(f"child exited early: {proc.returncode}")
                events = selector.select(timeout=0.25)
                if events:
                    ready = proc.stdout.readline().strip()
                    break
            selector.close()
            if ready != f"READY {mode} 32":
                raise RuntimeError(f"child readiness not proven: {ready!r}")
            # The only SIGKILL target is this exact *owned subprocess*.
            os.kill(proc.pid, signal.SIGKILL)
            proc.wait(timeout=5)
            if proc.returncode != -signal.SIGKILL:
                raise AssertionError(f"child exit {proc.returncode} not SIGKILL")
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.wait(timeout=5)
            if proc.stdout:
                proc.stdout.close()
            if proc.stderr:
                proc.stderr.close()
        recovery = subprocess.run(
            [str(binary), "restore", str(path)],
            cwd=ROOT, capture_output=True, text=True,
            timeout=10, check=True,
        )
        actual = int(recovery.stdout.strip().removeprefix("RECOVERED "))
        if actual != expected:
            raise AssertionError(
                f"{mode}: recovered {actual}, wanted {expected}; stderr={recovery.stderr}"
            )
        if mode == "flushed":
            # The atomic-write temp sibling is not authoritative.
            sibling = path.with_name(path.name + ".tmp")
            sibling.write_bytes(b"INCOMPLETE-SIMULATED-TEMP")
            recovered_with_tmp = subprocess.run(
                [str(binary), "restore", str(path)],
                cwd=ROOT, capture_output=True, text=True, timeout=10, check=True,
            )
            assert recovered_with_tmp.stdout.strip() == "RECOVERED 32"
            sibling.unlink()
            # Upstream deliberately treats its cache as OPTIONAL. A corrupt
            # primary causes a logged warning then a fresh, empty cache.
            # Our stricter HARS migration façade must instead refuse it.
            original = path.read_bytes()
            path.write_bytes(b"INVALID_PRIMARY_JSON")
            corrupt_upstream = subprocess.run(
                [str(binary), "restore", str(path)],
                cwd=ROOT, capture_output=True, text=True, timeout=10, check=True,
            )
            if corrupt_upstream.stdout.strip() != "RECOVERED 0":
                raise AssertionError("Unexpected Moli upstream corruption behavior")
            corrupt_strict = subprocess.run(
                [str(binary), "validate_strict", str(path)],
                cwd=ROOT, capture_output=True, text=True, timeout=10,
            )
            if corrupt_strict.returncode == 0:
                raise AssertionError("HARS strict façade accepted corrupt primary")
            if path.read_bytes() != b"INVALID_PRIMARY_JSON":
                raise AssertionError("Corrupt primary was overwritten unexpectedly")
            path.write_bytes(original)
            fixed_strict = subprocess.run(
                [str(binary), "validate_strict", str(path)],
                cwd=ROOT, capture_output=True, text=True, timeout=10, check=True,
            )
            assert fixed_strict.stdout.strip() == "STRICT_OK"
            fixed = subprocess.run(
                [str(binary), "restore", str(path)],
                cwd=ROOT, capture_output=True, text=True, timeout=10, check=True,
            )
            assert fixed.stdout.strip() == "RECOVERED 32"
        out.append({
            "scenario": mode, "child_killed_by_sigkill": True,
            "expected": expected, "recovered": actual,
            "upstream_corrupt_primary_discards_cache": mode == "flushed",
            "hars_strict_facade_rejects_corrupt_primary": mode == "flushed",
            "incomplete_temp_ignored": mode == "flushed",
            "path_type": "synthetic_private_runtime",
        })

result = {
    "project": "HA Rust Bridge", "scope": "synthetic_moli_process_sigkill",
    "power_loss_tested": False, "physical_fsync_tested": False,
    "haos_lab-guest_touched": False, "results": out,
}
print(json.dumps(result, sort_keys=True, indent=2))
