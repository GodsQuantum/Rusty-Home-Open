#!/usr/bin/env python3
"""Native App lifecycle evidence is generic and may NEVER authorize a write."""
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
src=(ROOT/"crates/supervisor-apps/src/operation_preflight.rs").read_text()
mod=(ROOT/"crates/supervisor-apps/src/lib.rs").read_text()
for term in (
    "assess_store_availability(catalog, detail, inventory, slug)?",
    "listed.installed != runtime.is_some()",
    "activation_authorized: false",
    "verified_backup_required",
    "LifecycleAction::Restore",
    "EvidenceStatus::VerifiedBackupRequired",
    "EvidenceStatus::SystemManaged",
    "EvidenceStatus::Inconsistent",
    "for n in 0..1017",
):
    assert term in src, term
for illegal in ("AppSupervisorClient", "SUPERVISOR_TOKEN", "TcpStream", ".post(", "AuditedPermit"):
    assert illegal not in src, illegal
assert "pub mod operation_preflight;" in mod
ci=(ROOT/".github/workflows/ci.yml").read_text()
assert "python3 scripts/test_lifecycle_preflight_contract.py" in ci
assert "cargo test --locked -p hars-supervisor-apps --lib operation_preflight" in ci
print("GENERIC_SUPERVISOR_LIFECYCLE_PREFLIGHT_CONTRACT_PASS")
