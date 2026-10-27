#!/usr/bin/env python3
"""P8dy protects genuine archive lineage and fail-closed official options."""
from pathlib import Path
r=Path(__file__).resolve().parents[1]
w=(r/'.github/workflows/p8dy-official-mosquitto-saved-options.yml').read_text()
h=(r/'scripts/supervisor_mosquitto_saved_options_lab.py').read_text()
t=(r/'scripts/test_p8dy_real_supervisor_mosquitto_saved_options.py').read_text()
for expected in (
 "haos_ova-18.3.qcow2.xz",
 "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
 "90fa213c3441e585d5a60387e9244d0ed38efaf8",
 "-snapshot -accel kvm",
 "p8do_official_mosquitto_qemu_pytest.py",
 "test_p8do_real_mosquitto_native_import.py",
 "test_p8dy_real_supervisor_mosquitto_saved_options.py",
 "P8DY_REAL_HAOS_SUPERVISOR_NATIVE_PERSISTED_MQTT_AUTH_OPTIONS_REFUSED_NO_BYPASS_PASS"):
 assert expected in w,expected
for expected in (
 'source=stage/"addon.json"', 'os.O_NOFOLLOW', 'st.st_nlink!=1',
 "MAX_ADDON_JSON=131072", "_unique_pairs", "_nonfinite",
 'metadata.get("user")','metadata.get("system")',
 'options=user.get("options")',
 "check_options(options)", '"local_auth_eligible":False',
 '"production_ready":False'):
 assert expected in h,expected
for expected in (
 "app-source-ingest", "app-source-verify", "app-source-stage-inactive",
 "core_mosquitto", "actual=read_staged_supervisor_options(private)",
 "assert not actual.get(\"logins\")",
 "P8DY_REAL_HAOS_MOSQUITTO_EMPTY_LOCAL_LOGINS_DELEGATED_AUTH_FAILS_CLOSED_PASS",
 "P8DY_MALICIOUS_DUPLICATE_NONFINITE_WRONG_LAYOUT_SYMLINK_REFUSED_PASS",
 "P8DY_REAL_HAOS_ARCHIVE_UNTOUCHED_ZERO_PRODUCTION_ACTION_PASS"):
 assert expected in t,expected
for banned in (
 "pull_request_target:", "workflow_dispatch:", "192.168.", "qm stop 100",
 "pct exec", "sudo podman", "ha supervisor options", "docker inspect hassio_cli",
 "SUPERVISOR_TOKEN", "http://172.30", "print(actual)", "print(options)"):
 assert banned not in w+h+t,banned
print("P8DY_OFFICIAL_SAVED_OPTIONS_SHA_BOUND_NO_PRIVILEGED_API_FAIL_CLOSED_SCOPE_PASS")
