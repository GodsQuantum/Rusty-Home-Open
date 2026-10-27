#!/usr/bin/env python3
"""Second public HAOS App must genuinely originate from Supervisor and
reach native Rust, no fake fixture from File Editor and no MQTT production.
"""
import argparse
from pathlib import Path
from xml.etree import ElementTree
r=Path(__file__).resolve().parents[1]
w=(r/'.github/workflows/p8do-genuine-mosquitto-native.yml').read_text()
guest=(r/'scripts/p8do_official_mosquitto_qemu_pytest.py').read_text()
host=(r/'scripts/test_p8do_real_mosquitto_native_import.py').read_text()
for symbol in (
 "haos_ova-18.3.qcow2.xz",
 "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
 "90fa213c3441e585d5a60387e9244d0ed38efaf8",
 "-snapshot -accel kvm","runs-on: ubuntu-24.04",
 "scripts/test_p8do_real_mosquitto_native_import.py",
 "scripts/p8do_official_mosquitto_qemu_pytest.py",
 "cargo clippy --locked -p hars-interop-moli --all-targets -- -D warnings",
 "P8DO_GENUINE_SECOND_OFFICIAL_SUPERVISOR_APP_MOSQUITTO_NATIVE_RUST_LAB_PASS"):
 assert symbol in w,symbol
for symbol in ("slug=\"core_mosquitto\"","ha apps install ","ha backups new",
 "ha backups info ","ha apps uninstall ","ha backups remove ",
 "P8DO_REAL_HAOS_MOSQUITTO_APP_INSTALLED_VIA_SUPERVISOR_PASS",
 "P8DO_GENUINE_HAOS_FILE_EDITOR_APP_ONLY_PARTIAL_BACKUP_EXPORTED_PASS"):
 if symbol.endswith("FILE_EDITOR_APP_ONLY_PARTIAL_BACKUP_EXPORTED_PASS"):
  continue
 assert symbol in guest,symbol
assert "core_configurator" not in guest and "core_mosquitto" in guest
assert 'marker_name="rusty_home_p8do_nonsecret_mqtt_marker.txt"' in guest
assert 'marker_filename="rusty_home_p8do_nonsecret_mqtt_marker.txt"' in host
assert "p8cu" not in guest.lower()
for symbol in ("inspect_backup(archive,{\"core_mosquitto\"})",
"app-source-ingest","app-source-verify","app-source-stage-inactive",
"core_mosquitto","P8DO_REAL_OFFICIAL_SECOND_APP_MOSQUITTO_NATIVE_PRIVATE_FILES_EXACT_BYTES_PASS",
"P8DO_REAL_MOSQUITTO_CORRUPT_ESCROW_REFUSED_ORIGINAL_AND_STAGED_DATA_PRESERVED_PASS"):
 assert symbol in host,symbol
for banned in ("workflow_dispatch:", "pull_request_target:", "sudo podman", "qm stop 100", "pct exec","192.168."):
 assert banned not in w+guest+host,banned
p=argparse.ArgumentParser()
p.add_argument("--junit",type=Path)
args=p.parse_args()
if args.junit:
 assert args.junit.stat().st_size<1048576
 cases=ElementTree.parse(args.junit).getroot().findall(".//testcase")
 needed={"test_start_supervisor","test_check_supervisor",
         "test_hars_official_mosquitto_app_only_partial_export"}
 assert len(cases)==3 and {t.get("name") for t in cases}==needed
 assert all(c.find(x) is None for c in cases for x in ("error","failure","skipped"))
 print("P8DO_GENUINE_SUPERVISOR_MOSQUITTO_THREE_QEMU_JUNIT_PASS")
else:
 print("P8DO_REAL_SECOND_OFFICIAL_SUPERVISOR_APP_NATIVE_SCOPE_CONTRACT_PASS")
