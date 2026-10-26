#!/usr/bin/env python3
from pathlib import Path
r=Path(__file__).resolve().parent.parent
s=(r/"crates/native-hub/src/haos_guest.rs").read_text()
w=(r/".github/workflows/p8ag-guest-service.yml").read_text()
x=(r/"crates/native-hub/src/bin/haos_guest_service_lab.rs").read_text()
for needle in ("run_guarded_guest_service_lab","private_vm_lock(private_dir)","verified_existing_overlay(spec, private_dir)","HARS_DISPOSABLE_KVM_LAB","UnixStream::connect","SERVICE_STOP","stop_guest(&mut child)"):
 assert needle in s,needle
for needle in ("P8AG_REAL_RUST_GUEST_SERVICE_QMP_REAP_PASS","HARS_LAB_AUTO_STOP_SECONDS=20","haos_ova-18.3.qcow2.xz"):
 assert needle in w,needle
assert "production_ready" in x and "--lab-service-run" in x
assert "hostfwd" not in w and "192.168." not in w
print("P8AG_OFFICIAL_RUST_GUEST_SERVICE_LAB_CONTRACT_PASS")
