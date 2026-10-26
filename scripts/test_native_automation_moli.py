#!/usr/bin/env python3
"""P3b: real Rust upstream Moli driver/engine after HA YAML -> Graph compile.

Synthetic sensor and actuator only; CI/isolated Linux lab, no Home Assistant/Python runtime.
"""
from __future__ import annotations
import json
import os
from pathlib import Path
import subprocess
ROOT = Path(__file__).resolve().parent.parent
if os.geteuid() == 0 or not (os.uname().nodename == "dev-build" or os.environ.get("GITHUB_ACTIONS") == "true"):
    raise SystemExit("P3b proof requires unprivileged isolated Linux lab or CI")
subprocess.run(["cargo", "build", "--locked", "--quiet", "-p", "hars-native-kernel"],
               cwd=ROOT, check=True, timeout=240)
binary = ROOT / "target/debug/hars-native-kernel"
proof = subprocess.run([str(binary), "--lab-ha-automation-moli"], cwd=ROOT,
                       capture_output=True, text=True, timeout=25)
assert proof.returncode == 0, f"native Moli YAML graph failed: {proof.stderr[-850:]}"
report = json.loads(proof.stdout)
assert report == {
    "compiled_from_ha_yaml": True,
    "actual_native_moli_automation": True,
    "required_human_approval": True,
    "initially_inactive": True,
    "synthetic_actuator_confirmed": True,
    "approval_invalidated_by_edit": True,
    "persisted_edit_inactive": True,
    "stale_approval_rejected": True,
    "no_run_after_edit": True,
    "restart_kept_edit_inactive": True,
    "no_python": True,
    "hardware_touched": False,
    "production_ready": False,
}, report
print("PASS: real native Moli Rust HA YAML->conditioned graph->human approval->fake actuator; persisted edit revocation, stale-approval denial and restart fence")
