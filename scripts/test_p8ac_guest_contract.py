#!/usr/bin/env python3
"""P8ac CI contract: genuine KVM and fail-closed Rust launcher, no radio."""
from pathlib import Path
b=Path(__file__).resolve().parent.parent
w=(b/".github/workflows/p8ac-rust-guest.yml").read_text()
s=(b/"crates/native-hub/src/haos_guest.rs").read_text()
cli=(b/"crates/native-hub/src/bin/haos_guest_lab.rs").read_text()
for required in (
    "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
    "haos_ova-18.3.qcow2.xz",
    "HARS_DISPOSABLE_KVM_LAB=1",
    "qemu-system-x86_64",
    "P8AC_NATIVE_RUST_REAL_OFFICIAL_HAOS_KVM_SERIAL_BOOT_PASS",
    "cargo clippy",
    "timeout -k 15s",
):
    assert required in w,required
for required in ("-snapshot", "snapshot=on", "-monitor", "-serial",
                 "homeassistant login:", "ImageDigestMismatch",
                 "O_NOFOLLOW", "GuestError::BootTimeout",
                 "source_image", "child.kill()", "read_only"):
    if required=="read_only":
        assert required in cli
    else:
        assert required in s,required
assert "192.168." not in w
assert "sudo -n pct" not in w
assert "hostfwd" not in w
assert "--device" not in w
print("P8AC_RUST_REAL_HAOS_GUEST_LAB_SECURITY_CONTRACT_PASS")
