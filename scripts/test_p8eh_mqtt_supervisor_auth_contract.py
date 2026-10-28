#!/usr/bin/env python3
"""P8eh experimental guard: even true loopback TCP auth is NOT HAOS auth migration."""
from pathlib import Path
root=Path(__file__).resolve().parents[1]
src=(root/"crates/mqtt-supervisor-auth-lab/src/lib.rs").read_text()
test=(root/"crates/mqtt-supervisor-auth-lab/tests/p8eh_mqtt_supervisor_auth.rs").read_text()
manifest=(root/"crates/mqtt-supervisor-auth-lab/Cargo.toml").read_text()
workspace=(root/"Cargo.toml").read_text()
supervisor=(root/"crates/supervisor-apps/src/lib.rs").read_text()
workflow=(root/".github/workflows/p8eh-mqtt-supervisor-delegated-auth-lab.yml").read_text()
assert 'name = "hars-mqtt-supervisor-auth-lab"' in manifest
assert '"crates/mqtt-supervisor-auth-lab"' in workspace
assert "mqtt_supervisor_auth_lab" not in supervisor
assert "mqtt-supervisor-auth-lab" not in (root/"crates/supervisor-apps/Cargo.toml").read_text()
for marker in (
    "POST /auth HTTP/1.1", "X-Supervisor-Token:",
    'path != "/authentication"', 'method != "POST"',
    'LabAuthApiPermission::FixtureGranted', "target.ip().is_loopback()",
    'path != "/authentication"', 'AuthDecision::Deny',
    "TcpStream::connect_timeout", "MAX_INCOMING", "MAX_WIRE",
    "fn parse_supervisor_reply", "deny_unknown_fields",
):
    assert marker in src,marker
for marker in (
    "P8EH_REAL_LOOPBACK_SUPERVISOR_POST_AUTH_ALLOW_DENY_TOKEN_HEADER_REDACTED_PASS",
    "P8EH_SUPERVISOR_UNREACHABLE_NO_PERMISSION_INVALID_TOKEN_EXTERNAL_EGRESS_DENIED_PASS",
    "P8EH_MQTT_SUPERUSER_ACL_RESERVED_DUPLICATE_JSON_AND_HTTP_PATH_DENY_PASS",
    "P8EH_FORGED_SUCCESS_REDIRECT_403_500_DUPLICATE_LENGTH_TRUNCATED_OVERSIZED_REFUSED_PASS",
    "P8EH_SUPERVISOR_TIMEOUT_FAIL_CLOSED_NO_CLIENT_SIDE_CREDENTIAL_CACHE_PASS",
):
    assert marker in test,marker
for forbidden in (
    "TcpListener::bind", "std::process::Command", "SUPERVISOR_TOKEN",
    "qm stop", "pct exec", "192.168.", "secrets.",
    "pull_request_target:", "workflow_dispatch:", "--privileged",
):
    assert forbidden not in src+workflow,forbidden
assert 'cargo check --locked -p hars-supervisor-apps --no-default-features' in workflow
print("P8EH_RUST_SUPERVISOR_DELEGATED_AUTH_LAB_ONLY_SCOPE_CONTRACT_PASS")
