#!/usr/bin/env python3
"""P8dj source contract: real HAOS->native App data, actual in-container HTTP.

This is *not* authenticated Home Assistant ingress; milestone6 stays blocked.
"""
from pathlib import Path
import argparse
from xml.etree import ElementTree
r=Path(__file__).resolve().parents[1]
script=(r/"scripts/test_p8dj_official_app_internal_http.py").read_text()
w=(r/".github/workflows/p8dj-official-app-internal-http.yml").read_text()
for marker in ("haos_ova-18.3.qcow2.xz",
 "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
 "90fa213c3441e585d5a60387e9244d0ed38efaf8",
 "-snapshot -accel kvm",
 "test_p8cu_genuine_file_editor_app_only_archive.py",
 "test_p8df_official_app_runtime.py",
 "test_p8dj_official_app_internal_http.py",
 "P8DJ_REAL_OFFICIAL_APP_RESTORED_BYTES_INTERNAL_HTTP_LAB_PASS"):
 assert marker in w,marker
for marker in ("app-source-ingest","app-source-stage-inactive",
 "ghcr.io/home-assistant/app-configurator:6.3.0",
 "P8DF_REAL_HAOS_SUPERVISOR_APP_DATA_NATIVE_RUST_PRIVATE_RESTAGE_PASS",
 '"--network","none"',
 '"podman", "exec", name, "python3"',
 "urllib.request.urlopen(url, timeout=3)",
 "http://127.0.0.1:8099/",
 '"{{.HostConfig.ReadonlyRootfs}}"]).stdout.strip()=="false"',
 "P8DJ_REAL_UPSTREAM_CONFIGURATOR_HTML_OK",
 "P8DJ_OFFICIAL_APP_NO_SUPERVISOR_API_FAILS_CLOSED_NO_HTML_PASS",
 '"podman","port",name',
 "RUNNER_TEMP","GITHUB_ACTIONS"):
 assert marker in script,marker
for bad in ("sudo podman", "--network=host", "--privileged",
 "pull_request_target:", "workflow_dispatch:", "192.168.", "pct exec", "qm stop 100"):
 assert bad not in script+w,bad
assert "ingress" in (r/"docs/PRODUCTION_CERTIFICATION_MASTERPLAN_2026-10-24.md").read_text().lower()
guest=(r/"scripts/p8dj_official_supervisor_http_pytest.py").read_text()
for required in ("ha apps start ","ha apps info ",
 "docker exec hassio_supervisor python3 -c ",
 "http://core-configurator:8099/",
 "P8DJ_REAL_HAOS_SUPERVISOR_LAUNCHED_OFFICIAL_EDITOR_APP_PASS",
 "P8DJ_REAL_OFFICIAL_SUPERVISOR_TRUSTED_INTERNAL_HTTP_200_HTML_PASS",
 "ha backups new", "ha apps uninstall"):
 assert required in guest,required
assert "p8dj_official_supervisor_http_pytest.py" in w
assert "test_hars_official_file_editor_supervisor_internal_http" in w
assert "P8DJ_OFFICIAL_APP_NO_SUPERVISOR_API_FAILS_CLOSED_NO_HTML_PASS" in script
parser=argparse.ArgumentParser()
parser.add_argument("--junit",type=Path)
args=parser.parse_args()
if args.junit:
 assert args.junit.is_file() and args.junit.stat().st_size<1024*1024
 cases=ElementTree.parse(args.junit).getroot().findall(".//testcase")
 expected={"test_start_supervisor","test_check_supervisor",
           "test_hars_official_file_editor_supervisor_internal_http"}
 assert len(cases)==3 and {n.get("name") for n in cases}==expected
 assert not any(n.find(t) is not None for n in cases for t in ("failure","error","skipped"))
 print("P8DJ_REAL_OFFICIAL_HAOS_SUPERVISOR_HTTP_KVM_THREE_JUNIT_PASS")
else:
 print("P8DJ_REAL_SUPERVISOR_HTTP_POSITIVE_AND_STANDALONE_DENIAL_SCOPE_PASS")
