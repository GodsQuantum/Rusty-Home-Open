#!/usr/bin/env python3
"""No double owner: hold flock in a separate process; Rust must fail before QEMU."""
import argparse,fcntl,os,subprocess
from pathlib import Path
parser=argparse.ArgumentParser()
for opt in ("--binary","--source","--sha","--dir"):parser.add_argument(opt,required=True)
args=parser.parse_args()
lock=Path(getattr(args,"dir"))/".single-owner-lock"
with lock.open("r+b") as stream:
    fcntl.flock(stream,fcntl.LOCK_EX|fcntl.LOCK_NB)
    p=subprocess.run([args.binary,"--lab-resume-marker","/usr/bin/qemu-system-x86_64",
        "/usr/share/ovmf/OVMF.fd",args.source,args.sha,getattr(args,"dir")],
        env={**os.environ,"HARS_DISPOSABLE_KVM_LAB":"1"},text=True,capture_output=True,timeout=12)
    assert p.returncode!=0 and "HAOS_GUEST_LAB_FAIL_CLOSED" in p.stderr
    assert not p.stdout.strip()
    fcntl.flock(stream,fcntl.LOCK_UN)
print("P8AF_INDEPENDENT_RUST_MANAGER_FLOCK_DOUBLE_OWNER_DENIED_PASS")
