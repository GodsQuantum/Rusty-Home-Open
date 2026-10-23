#!/usr/bin/env python3
"""Refuse turning Core corroboration into a default-build write authority."""
from pathlib import Path
root=Path(__file__).resolve().parent.parent
crate=(root/"crates/supervisor-apps").joinpath("Cargo.toml").read_text()
source=(root/"crates/supervisor-apps/src/core_corroborated_permit_lab.rs").read_text()
lib=(root/"crates/supervisor-apps/src/lib.rs").read_text()
transport=(root/"crates/supervisor-apps/src/official_transport.rs").read_text()
ci=(root/".github/workflows/ci.yml").read_text()
assert 'core-corroborated-preview-lab = ["core-user-live-readonly", "persistent-session-lab"]' in crate
assert '#[cfg(feature = "core-corroborated-preview-lab")]' in lib
for must in ("ingress_user_claim_from_stream(accepted_tcp",
             "fetch_official_core_users_readonly(now)",
             "CoreLiveResult::PermissionDenied => return Ok(None)",
             "CoreAccountStatus::ActiveAdmin",
             "actor.subject() != claim.as_str()",
             "actor.role() != Role::Administrator",
             "current_fixture_actor(session_token, now)",
             "grant_recorded_session("):
    assert must in source,must
for forbidden in ("preview_execute(", "TcpListener::bind(", "POST /",
                  "grant_recorded(", "HARS_P8H_QEMU_ONLY"):
    assert forbidden not in source,forbidden
assert 'pub fn preview_execute(' in transport
assert '#[cfg(feature = "managed-apps-preview")]' in transport
assert "cargo test --locked -p hars-supervisor-apps --features core-corroborated-preview-lab --lib core_corroborated_permit_lab" in ci
print("CORE_CORROBORATED_PREVIEW_FAIL_CLOSED_CONTRACT_PASS")
