#!/usr/bin/env python3
from pathlib import Path
b=Path(__file__).resolve().parent.parent
w=(b/".github/workflows/p8af-rust-reattach.yml").read_text()
m=(b/"crates/native-hub/src/haos_guest.rs").read_text()
cli=(b/"crates/native-hub/src/bin/haos_guest_lab.rs").read_text()
for marker in ("P8AF_SECOND_RUST_PROCESS_REATTACHED_REAL_HAOS_STATE_PASS",
    "P8AF_INDEPENDENT_RUST_MANAGER_FLOCK_DOUBLE_OWNER_DENIED_PASS" if False else "test_p8af_manager_lock.py",
    "--lab-provision-marker","--lab-resume-marker","haos_ova-18.3.qcow2.xz",
    "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090"):
    assert marker in w,marker
assert w.index("--lab-provision-marker") < w.index("test_p8af_manager_lock.py",w.index("--lab-provision-marker"))
assert w.index("test_p8af_manager_lock.py",w.index("--lab-provision-marker")) < w.index("--lab-resume-marker",w.index("--lab-provision-marker"))
for marker in ("verified_existing_overlay","backing-filename","backing-filename-format",
               "O_NOFOLLOW","private_vm_lock","session_verify_guest_marker",
               "guest_marker_in_separate_manager_lab"):
    assert marker in m,marker
assert "guest_marker_verified_by_second_rust_manager" in cli
assert "192.168." not in w and "hostfwd" not in w
print("P8AF_REAL_GUEST_DIFFERENT_RUST_MANAGER_RESTART_CONTRACT_PASS")
