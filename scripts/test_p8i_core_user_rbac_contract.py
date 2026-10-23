#!/usr/bin/env python3
"""Guard real-feature separation: Core/Ingress policy stays non-writing."""
from pathlib import Path
import tomllib

ROOT = Path(__file__).resolve().parent.parent
package = tomllib.loads((ROOT/"crates/supervisor-apps/Cargo.toml").read_text())
features = package["features"]
assert features["core-user-rbac-lab"] == [
    "core-user-snapshot-lab","managed-apps-preview"
]
assert "core-user-rbac-lab" not in features.get("default", [])
lib = (ROOT/"crates/supervisor-apps/src/lib.rs").read_text()
assert '#[cfg(feature = "core-user-rbac-lab")]' in lib
src = (ROOT/"crates/supervisor-apps/src/core_user_rbac_lab.rs").read_text()
assert "core.status(ingress, now)?" in src
assert "policy.evaluate(actor.as_ref(), request, None)" in src
for disallowed in ("TcpListener", "TcpStream", "SUPERVISOR_TOKEN",
                   "AppSupervisorClient", ".post(", "Ack::sign"):
    assert disallowed not in src
ci = (ROOT/".github/workflows/ci.yml").read_text()
assert "cargo test --locked -p hars-supervisor-apps --features core-user-rbac-lab --lib" in ci
assert "cargo clippy --locked -p hars-supervisor-apps --all-targets --features core-user-rbac-lab" in ci
assert "python3 scripts/test_p8i_core_user_rbac_contract.py" in ci
assert "P8I_REAL_CORE_WS_USERS_READ" in (ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
print("CORE_INGRESS_RBAC_PREVIEW_CONTRACT_PASS")
