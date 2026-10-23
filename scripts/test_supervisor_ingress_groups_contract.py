#!/usr/bin/env python3
"""Native Supervisor groups from trusted ingress only; never an App write permit."""
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent
source = (ROOT/"crates/control-plane/src/ingress.rs").read_text()
server = (ROOT/"crates/control-plane/src/main.rs").read_text()
html = (ROOT/"web/index.html").read_text()
smoke = (ROOT/"scripts/smoke-supervisor-ingress.sh").read_text()
kvm = (ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
config = (ROOT/"packaging/product-app/rusty_home/config.yaml").read_text()
for key in ("x-remote-user-groups", '"system-admin"', "is_admin = true",
            "HashSet", "IngressError::DuplicateHeader", "IngressError::UntrustedPeer"):
    assert key in source, key
for key in ("GET /api/session", "actions_authorized", "supervisor_session_response",
            "AccessMode::SupervisorIngress", "identity.is_admin"):
    assert key in server, key
assert "/api/session" in smoke
assert '"api/session"' in kvm
assert 'fetch("api/session",{cache:"no-store"})' in html
assert 'id="session-status"' in html
assert "panel_admin: true" in config
assert "homeassistant_api: false" in config
assert '"POST /api/session"' not in server
assert "actions_authorized\":false" in server or '"actions_authorized":false' in server
assert (ROOT/"docs/SUPERVISOR_INGRESS_NATIVE_SESSION_2026-10-23.md").is_file()
print("NATIVE_SUPERVISOR_INGRESS_GROUPS_CONTRACT_PASS")
