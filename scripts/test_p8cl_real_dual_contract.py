#!/usr/bin/env python3
"""P8cl strict contract: official HAOS 18.3 source, genuinely two disabled files."""
from pathlib import Path
from xml.etree import ElementTree as ET
import argparse
r=Path(__file__).resolve().parent.parent
w=(r/".github/workflows/p8cl-real-haos-dual-disabled.yml").read_text()
g=(r/"scripts/p8cl_real_haos_dual_qemu_pytest.py").read_text()
h=(r/"scripts/test_p8cl_real_haos_dual_backup.py").read_text()
y=(r/"scripts/fixtures/p8cl_two_inactive_haos_automations.yaml").read_text()
for t in ("haos_ova-18.3.qcow2.xz","fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090","90fa213c3441e585d5a60387e9244d0ed38efaf8","-snapshot -accel kvm","/dev/kvm","test_hars_dual_inactive_automation_source","test_p8cl_real_haos_dual_backup.py","runs-on: ubuntu-24.04"):
 assert t in w,t
for t in ("ha backups new","ha backups info","protected","core.entity_registry","binary_sensor.lab_door","light.lab_entry","P8CL_TWO_INACTIVE_YAML_BYTES_INSIDE_ACTUAL_SUPERVISOR_BACKUP_PASS","curl --fail"):
 assert t in g,t
for t in ("inspect_backup","import-unprotected-backup","--lab-ha-backup-automations-preview","explicitly_disabled_count","==6","actions_executed","device_commands","rollback","P8CL_GENUINE_HAOS_SUPERVISOR_TWO_INACTIVE_YAML_RUST_SHA_BOUND_PREVIEW_PASS"):
 assert t in h,t
for cli in (
 "python3 scripts/test_p8aa_backup_to_native_stage.py --binary target/debug/ha_registry_stager",
 "python3 scripts/test_p8be_automation_source.py --binary target/debug/ha_registry_stager",
 "python3 scripts/test_p8cl_real_haos_dual_backup.py --archive",
):
 assert cli in w,cli
assert y.count("  initial_state: false")==2
assert "initial_state: true" not in y
assert y.count("\n- id: ")==1 and y.startswith("- id: ")
for forbidden in ("192.168.","VM100","--privileged","sudo podman","workflow_dispatch:"):
 assert forbidden not in w+g+h,forbidden
p=argparse.ArgumentParser()
p.add_argument("--junit",type=Path)
a=p.parse_args()
if a.junit:
 assert a.junit.is_file() and a.junit.stat().st_size < 1024*1024
 rows=ET.parse(a.junit).getroot().findall(".//testcase")
 assert len(rows)==3 and {e.get("name") for e in rows}=={"test_start_supervisor","test_check_supervisor","test_hars_dual_inactive_automation_source"}
 assert not any(e.find(k) is not None for e in rows for k in ("error","failure","skipped"))
 print("P8CL_REAL_HAOS_KVM_JUNIT_THREE_PASS")
else:
 print("P8CL_REAL_HAOS_DUAL_INACTIVE_SOURCE_SAFETY_CONTRACT_PASS")
