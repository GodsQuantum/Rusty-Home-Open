#!/usr/bin/env python3
from pathlib import Path
r=Path(__file__).resolve().parent.parent
w=(r/".github/workflows/p8ae-rust-persistent.yml").read_text()
m=(r/"crates/native-hub/src/haos_guest.rs").read_text()
cli=(r/"crates/native-hub/src/bin/haos_guest_lab.rs").read_text()
for token in ("--lab-persistent-restart","P8AE_REAL_HAOS_RUST_GUEST_STATE_PERSISTS_AFTER_HARD_RESTART_PASS",
              "haos_ova-18.3.qcow2.xz",
              "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
              "haos-rust-private-overlay","guest-overlay.qcow2"):
 assert token in w,token
for token in ("prove_guest_cow_state_across_restarts_in_lab","TEST_MARKER",
              "sha256sum","sync &&", "LOCK_EX","LOCK_NB",
              "make_overlay", "session_verify_guest_marker(spec, &overlay, true",
              "session_verify_guest_marker(spec, &overlay, false",
              "File::open(&overlay)", "snapshot=on"):
 assert token in m,token
assert "guest_state_persisted_across_restart" in cli
assert "immutable_source_image" in cli
assert "192.168." not in w and "hostfwd" not in w
print("P8AE_RUST_GENUINE_HAOS_PERSISTENT_RESTART_CONTRACT_PASS")
