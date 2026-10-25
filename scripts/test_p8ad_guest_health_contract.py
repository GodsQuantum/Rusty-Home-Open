#!/usr/bin/env python3
"""P8ad guards that real Core and Supervisor replies, not serial boot, are required."""
from pathlib import Path
r=Path(__file__).resolve().parent.parent
w=(r/".github/workflows/p8ad-rust-health.yml").read_text()
code=(r/"crates/native-hub/src/haos_guest.rs").read_text()
cli=(r/"crates/native-hub/src/bin/haos_guest_lab.rs").read_text()
for marker in ("--lab-supervisor-core-health",
 "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
 "P8AD_NATIVE_RUST_OFFICIAL_SUPERVISOR_AND_CORE_LIVE_HEALTH_PASS",
 "official_supervisor_cli_ok", "official_core_cli_ok"):
 assert marker in w,marker
for marker in ("ha supervisor info --no-progress --raw-json",
 "ha core info --no-progress --raw-json", "GuestHealthTimeout",
 "stdin(Stdio::piped())", "sha256_file(&spec.source_image)",
 "CLI_OK_SPACED","GuestError::InvalidArguments"):
 assert marker in code,marker
assert "--lab-boot-ephemeral" in cli and "--lab-supervisor-core-health" in cli
assert "192.168." not in w and "hostfwd" not in w
assert "production_ready" in cli
print("P8AD_REAL_OFFICIAL_HAOS_SUPERVISOR_CORE_LAB_CONTRACT_PASS")
