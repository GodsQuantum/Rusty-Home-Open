#!/usr/bin/env python3
"""P8dl verifies REAL Core ingress rejects unauthenticated or spoofed requests."""
import argparse
from pathlib import Path
from xml.etree import ElementTree
r=Path(__file__).resolve().parents[1]
g=(r/"scripts/p8dl_official_core_ingress_denial_pytest.py").read_text()
w=(r/".github/workflows/p8dl-real-core-ingress-denial.yml").read_text()
for required in (
 "ha apps install ", "ha apps start ", "ha apps uninstall ",
 "ha backups new ", "P8DJ_REAL_OFFICIAL_SUPERVISOR_TRUSTED_INTERNAL_HTTP_200_HTML_PASS",
 "P8DL_REAL_CORE_SUPERVISOR_INGRESS_MISSING_FORGED_AND_SPOOFED_HEADER_DENIED_PASS",
 "P8DL_OFFICIAL_FILE_EDITOR_CORE_INGRESS_FAIL_CLOSED_WITHOUT_BROWSER_SESSION_PASS",
 "http://homeassistant:8123", "ingress_session=invalid-lab-session-value",
 "X-Remote-User-Id", "X-Remote-User-Groups",
 'outcome=shell.run_check("docker exec hassio_supervisor python3 "',
 "shell.run_check", "72", "guest_container",
):
 assert required in g,required
for required in (
 "p8dl_official_core_ingress_denial_pytest.py",
 "test_hars_official_file_editor_ingress_unauthenticated_denial",
 "P8DL_REAL_OFFICIAL_HAOS_CORE_INGRESS_NEGATIVE_SESSION_TEST_PASS",
 "haos_ova-18.3.qcow2.xz",
 "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
 "-snapshot -accel kvm","runs-on: ubuntu-24.04",
):
 assert required in w,required
for blocked in ("pull_request_target:", "workflow_dispatch:", "sudo podman",
                "pct exec","qm stop 100","192.168."):
 assert blocked not in w+g,blocked
p=argparse.ArgumentParser();p.add_argument("--junit",type=Path)
args=p.parse_args()
if args.junit:
 assert args.junit.is_file() and args.junit.stat().st_size<1048576
 cases=ElementTree.parse(args.junit).getroot().findall(".//testcase")
 expected={"test_start_supervisor","test_check_supervisor",
           "test_hars_official_file_editor_ingress_unauthenticated_denial"}
 assert len(cases)==3 and {c.get("name") for c in cases}==expected
 assert all(all(c.find(tag) is None for tag in ("error","failure","skipped")) for c in cases)
 print("P8DL_REAL_OFFICIAL_HAOS_CORE_INGRESS_DENIAL_THREE_JUNIT_PASS")
else:
 print("P8DL_REAL_CORE_SUPERVISOR_APP_INGRESS_DENIAL_SCOPE_CONTRACT_PASS")
