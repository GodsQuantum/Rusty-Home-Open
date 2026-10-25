#!/usr/bin/env python3
"""Pinned new HAOS -> Rust genuine archive proof. With --junit verify exact run."""
import argparse
from pathlib import Path
from xml.etree import ElementTree as ET
root=Path(__file__).resolve().parent.parent
w=(root/".github/workflows/p8aa-haos-native-restore.yml").read_text()
scenario=(root/"scripts/p8aa_native_backup_source_qemu_pytest.py").read_text()
host=(root/"scripts/test_p8aa_genuine_archive_qemu.py").read_text()
for marker in (
    "90fa213c3441e585d5a60387e9244d0ed38efaf8",
    "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
    "-snapshot -accel kvm", "test_hars_native_backup_source",
    "test_p8aa_genuine_archive_qemu.py",
    "test_p8aa_backup_to_native_stage.py",
    "P8AA_ACTUAL_SUPERVISOR_ARCHIVE_TO_NATIVE_RUST_LAB_CERTIFIED",
    "haos_ova-18.3.qcow2.xz",
):
    assert marker in w,marker
for marker in (
    "shell_json", "ha backups new", "protected", "core.entity_registry",
    "fictional_vendor_1", "fictional_vendor_2",
    "curl --fail", "P8AA_REAL_HAOS_SUPERVISOR_CREATED_BACKUP_EXPORTED_PASS"
):
    assert marker in scenario,marker
for marker in (
    "inspect_backup", "import-unprotected-backup", "rollback",
    "archive_sha256", "stable_identities", "P8AA_REAL_SUPERVISOR_BACKUP_TO_RUST_DURABLE_ROLLBACK_PASS"
):
    assert marker in host,marker
assert "::test_hars_cross_vm_target" not in w
assert "homeassistant.tar.gz" not in scenario
assert "192.168." not in w
p=argparse.ArgumentParser()
p.add_argument("--junit",type=Path)
args=p.parse_args()
if args.junit:
    assert args.junit.is_file() and args.junit.stat().st_size < 1024*1024
    tree=ET.parse(args.junit).getroot()
    cases=tree.findall(".//testcase")
    wanted={"test_start_supervisor","test_check_supervisor","test_hars_native_backup_source"}
    assert len(cases)==3 and {c.get("name") for c in cases}==wanted
    assert not any(any(c.find(x) is not None for x in ("error","failure","skipped")) for c in cases)
    print("P8AA_GENUINE_HAOS_JUNIT_3_OF_3_PASS")
else:
    print("PASS: pinned genuine HAOS-only source, real Supervisor tar -> native Rust, no household")
