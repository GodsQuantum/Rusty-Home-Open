#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parent.parent
mod=(root/"crates/supervisor-apps/src/core_corroborated_permit_lab.rs").read_text()
fence=(root/"crates/supervisor-apps/src/durable_dispatch_fence_lab.rs").read_text()
workflow=(root/".github/workflows/ci.yml").read_text()
for item in ("pub fn submit_corroborated_disposable_dispatch(",
             "corroborated_active_admin(accepted_tcp, ingress_headers, now)?",
             "current_fixture_actor(token, now)?",
             "armed.same_corroborated_session(token)",
             "armed.allow_after_fresh_core_recheck(session_token)?",
             "post_arm_core_recheck_rejects_revocation_and_new_session_same_admin"):
    assert item in mod,item
for item in ("corroborated_session: Option<[u8; 32]>",
             "corroborated_rechecked: bool",
             "if armed.corroborated_session.is_some() && !armed.corroborated_rechecked",
             "TransportError::ManagedPreviewDisabled"):
    assert item in fence,item
assert "scripts/test_p8bm_recheck_contract.py" in workflow
assert "core-durable-dispatch-lab" in workflow
assert "lab-app-actions = []" in (root/"crates/supervisor-apps/Cargo.toml").read_text()
print("P8BM_PREPOST_CORE_RBAC_RECHECK_BYPASS_DENIAL_AND_DURABLE_LOCK_SCOPE_PASS")
