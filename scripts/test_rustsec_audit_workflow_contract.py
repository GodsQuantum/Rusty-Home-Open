#!/usr/bin/env python3
"""Contract: independently audit BOTH lockfiles, with no silent vulnerability exemptions."""
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ci = (ROOT / ".github/workflows/rustsec-locked-audit.yml").read_text()
for token in (
    "pull_request:",
    "schedule:",
    "workflow_dispatch:",
    "contents: read",
    "persist-credentials: false",
    "cargo install cargo-audit --version 0.22.2 --locked",
    "cargo audit --file Cargo.lock --deny warnings",
    "cargo audit --file fuzz/Cargo.lock --deny warnings",
    "python3 scripts/check_release_gate.py --expect-blocked",
):
    assert token in ci, f"missing RustSec fail-closed workflow contract: {token}"
assert "pull_request_target:" not in ci
assert "--ignore" not in ci and "--no-fetch" not in ci and "audit fix" not in ci
assert (ROOT / "Cargo.lock").is_file()
assert (ROOT / "fuzz/Cargo.lock").is_file()
print("RUSTSEC_AUDIT_CONTRACT_PASS: two immutable lockfiles, no exclusions, no auto-fix")
