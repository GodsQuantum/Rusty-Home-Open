#!/usr/bin/env python3
"""Contract guard: durable lab sessions are bounded and never compiled by default."""
from pathlib import Path
import tomllib
ROOT = Path(__file__).resolve().parent.parent
pkg = tomllib.loads((ROOT/"crates/supervisor-apps/Cargo.toml").read_text())
assert "persistent-session-lab" not in pkg["features"].get("default", [])
source = (ROOT/"crates/supervisor-apps/src/persistent_session_lab.rs").read_text()
for guard in (
    "MAX_SESSIONS: u64 = 1_024",
    "MAX_APPROVALS: u64 = 4_096",
    "MAX_APPROVALS_PER_SESSION: usize = 32",
    "table.len().map_err(error)? >= MAX_SESSIONS",
    "approvals.len().map_err(error)? >= MAX_APPROVALS",
    "old.owner_hash == key",
    "approval.owner_hash == key",
    "tx.set_two_phase_commit(true)",
):
    assert guard in source, guard
tests = (ROOT/"crates/supervisor-apps/tests/p8i_durable_sessions.rs").read_text()
assert "durable_approvals_are_bounded_pruned_and_atomically_revoked" in tests
assert "durable_sessions_reject_capacity_then_reuse_expired_slots" in tests
ci = (ROOT/".github/workflows/ci.yml").read_text()
assert "cargo test --locked -p hars-supervisor-apps --features persistent-session-lab --test p8i_durable_sessions" in ci
assert "cargo clippy --locked -p hars-supervisor-apps --all-targets --features persistent-session-lab" in ci
assert "python3 scripts/test_p8i_durable_contract.py" in ci
print("P8I_DURABLE_BOUNDED_REVOKE_CONTRACT_PASS")
