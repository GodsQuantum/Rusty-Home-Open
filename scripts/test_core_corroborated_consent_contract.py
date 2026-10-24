#!/usr/bin/env python3
"""The one-use approval challenge requires a newly verified Core account."""
from pathlib import Path
root=Path(__file__).resolve().parent.parent
code=(root/"crates/supervisor-apps/src/core_corroborated_permit_lab.rs").read_text()
ci=(root/".github/workflows/ci.yml").read_text()
name="pub fn issue_corroborated_disposable_confirmation("
assert code.count(name)==1
start=code.index(name)
end=code.index("fn issue_checked_fixture_confirmation(",start)
source=code[start:end]
for required in ("corroborated_active_admin(accepted_tcp, ingress_headers, now)",
                 "issue_checked_fixture_confirmation(", "return Ok(None)"):
    assert required in source,required
issue=code[end:code.index("/// Purely internal test seam:",end)]
for required in ("status", "CoreAccountStatus::ActiveAdmin",
                 "sessions.current_fixture_actor(token, now)",
                 "actor.subject() != claim.as_str()",
                 "actor.role() != Role::Administrator",
                 ".issue_confirmation(policy, token, request, now)"):
    assert required in issue,required
assert "python3 scripts/test_core_corroborated_consent_contract.py" in ci
assert "preview_execute(" not in source
assert "POST " not in source
print("CORE_CORROBORATED_ONE_USE_CONSENT_CONTRACT_PASS")
