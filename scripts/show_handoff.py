#!/usr/bin/env python3
"""Live, read-only handoff from currently checked out Git + gate registry."""
from pathlib import Path
import json
import subprocess

ROOT = Path(__file__).resolve().parent.parent
gates = json.loads((ROOT / "config/release-gates.json").read_text())["required_gates"]
evidence = json.loads((ROOT / "config/certification-evidence-registry.json").read_text())

def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True, timeout=5).strip()

states = {status: sum(x["status"] == status for x in gates)
          for status in ("lab_partial", "blocked", "certified")}
print("RUSTY_HOME_LIVE_HANDOFF")
print("HEAD", git("rev-parse", "HEAD"))
print("BRANCH", git("branch", "--show-current") or "(detached)")
print("DIRTY", bool(git("status", "--porcelain")))
print("RELEASE", states, "TOTAL", len(gates), "DEPLOYMENT_AUTHORIZED", False)
print("SAME_VM_HAOS_EVIDENCE", evidence["verification"]["run_url"])
print("FRESH_SECOND_VM_HAOS_EVIDENCE", evidence["cross_vm_verification"]["run_url"])
print("HAOS_TO_NATIVE_RUST_MIGRATION_NOT_CERTIFIED")
print("READ docs/HANDOFF_CURRENT.md docs/WORK_LOG.md docs/CERTIFICATION_EVIDENCE_REGISTER.md")
print("READ docs/PRODUCTION_CERTIFICATION_MASTERPLAN_2026-10-24.md")
