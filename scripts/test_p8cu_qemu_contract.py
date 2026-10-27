#!/usr/bin/env python3
"""P8cu fixed benign App-only HAOS KVM and fail-closed source contract."""
import argparse
from pathlib import Path
from xml.etree import ElementTree as ET
r=Path(__file__).resolve().parent.parent
w=(r/".github/workflows/p8cu-official-app-only-preflight.yml").read_text()
g=(r/"scripts/p8cu_official_file_editor_partial_qemu_pytest.py").read_text()
h=(r/"scripts/test_p8cu_genuine_file_editor_app_only_archive.py").read_text()
pre=(r/"scripts/inspect_ha_backup.py").read_text()
for token in (
 "haos_ova-18.3.qcow2.xz",
 "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
 "90fa213c3441e585d5a60387e9244d0ed38efaf8",
 "test_hars_official_file_editor_app_only_partial_export",
 "scripts/p8p_cross_vm_backup_relay.py",
 "scripts/test_p8cu_genuine_file_editor_app_only_archive.py",
 "scripts/test_p8cu_partial_app_preflight.py",
 "-snapshot -accel kvm",
 "runs-on: ubuntu-24.04",
):
 assert token in w,token
for token in (
 "ha apps install ","ha backups new","--app ","ha backups info",
 "version_latest", "ha store reload --no-progress --raw-json", "ha core info",
 "ha supervisor options --auto-update --no-progress --raw-json",
 "ha supervisor reload --no-progress --raw-json",
 "P8CU_DISPOSABLE_HAOS_UPSTREAM_SUPERVISOR_PLUGIN_POLICY_READY_PASS",
 "_ensure_disposable_plugins_current(shell_json, _sleep)",
 "ha backups remove","ha apps uninstall",
 "core_configurator","sha256sum","X-Hars-Slug:","X-Hars-Boot:",
 "P8CU_GENUINE_HAOS_FILE_EDITOR_APP_ONLY_PARTIAL_BACKUP_EXPORTED_PASS",
):
 assert token in g,token
for token in (
 "inspect_backup(", "expected={\"core_configurator\"}",
 "app_only_partial_archive_identified", "tarfile.open(",
 "normalized_archive_name(", "P8CU_GENUINE_OFFICIAL_SUPERVISOR_ADDON_BYTES_EXACT_PRIVATE_MARKER_PASS",
 "can_restore", "eligible_for_migration",
):
 assert token in h,token
for token in (
 "backup_type == \"partial\" and not home_part and bool(addons)",
 "app_only_partial_requires_explicit_unprotected",
 "app_only_partial_requires_all_addon_payloads",
):
 assert token in pre,token
assert (r/"scripts/test_p8cu_partial_app_preflight.py").is_file()
for forbidden in ("qm stop 100","192.168.","sudo podman","pct exec","workflow_dispatch:", "pull_request_target:"):
 assert forbidden not in g+h+w,forbidden
p=argparse.ArgumentParser()
p.add_argument("--junit",type=Path)
a=p.parse_args()
if a.junit:
 assert a.junit.is_file() and a.junit.stat().st_size<1024*1024
 cases=ET.parse(a.junit).getroot().findall(".//testcase")
 wanted={"test_start_supervisor","test_check_supervisor",
         "test_hars_official_file_editor_app_only_partial_export"}
 assert len(cases)==3 and {x.get("name") for x in cases}==wanted,[(x.get("name")) for x in cases]
 assert not any(x.find(t) is not None for x in cases for t in ("failure","error","skipped"))
 print("P8CU_REAL_HAOS_FILE_EDITOR_KVM_THREE_JUNIT_PASS")
else:
 print("P8CU_GENUINE_FILE_EDITOR_PARTIAL_OFFLINE_SCOPE_CONTRACT_PASS")
