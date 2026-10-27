#!/usr/bin/env python3
"""Evidence contract: actual upstream MQTT good/wrong/anonymous checks."""
from pathlib import Path
r=Path(__file__).resolve().parents[1]
w=(r/'.github/workflows/p8dt-mosquitto-local-auth.yml').read_text()
t=(r/'scripts/test_p8dt_mosquitto_supervisor_local_auth.py').read_text()
h=(r/'scripts/haos_mosquitto_local_auth_lab.py').read_text()
for x in (
 "haos_ova-18.3.qcow2.xz",
 "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
 "90fa213c3441e585d5a60387e9244d0ed38efaf8",
 "-snapshot -accel kvm","runs-on: ubuntu-24.04",
 "p8do_official_mosquitto_qemu_pytest.py",
 "test_p8do_real_mosquitto_native_import.py",
 "test_p8dt_mosquitto_supervisor_local_auth.py",
 "pexpect==4.9.0",
 "P8DT_REAL_HAOS_NATIVE_MQTT_LOCAL_AUTH_GOOD_BAD_ANON_DENIAL_PASS"):
 assert x in w,x
for x in (
 "password_pre_hashed", "require_certificate",
 'customization.get("active") is not False',
 "username.casefold() in RESERVED",
 '"allow_anonymous false"', "pexpect.spawn",
 "p.sendline(password)", '"--entrypoint","mosquitto_passwd"',
 '"--network","none"','"--cap-drop","ALL"',
 '"--userns=keep-id"',
 '"--security-opt","no-new-privileges"',
 '"password_file /mosquitto/config/passwords"'
):
 assert x in h,x
for x in ("app-source-stage-inactive","core_mosquitto",
    "P8DT_REAL_NATIVE_MQTT_GOOD_LOGIN_ACCEPTED_ANONYMOUS_AND_BAD_DENIED_PASS",
    "P8DT_SUPERVISOR_UNKNOWN_UNMAPPED_TLS_DELEGATED_AUTH_OPTIONS_REFUSED_PASS",
    "P8DT_OFFICIAL_SUPERVISOR_LOCAL_OPTION_FORMAT_TO_PRIVATE_UPSTREAM_PASSWORD_HASH_PASS",
    "P8DT_REAL_AUTH_MQTT_RETAINED_MESSAGE_AFTER_SECOND_OCI_PASS",
    "P8DT_NO_HOST_PORT_REAL_SUPERVISOR_SOURCE_UNCHANGED_PASS",
    "mosquitto_pub","mosquitto_sub",
    '"--network","none"', "P8DT_THIS_IS_WRONG_PASSWORD"):
 assert x in t,x
for bad in ("--network=host","--privileged","sudo podman","pct exec","qm stop 100",
            "192.168.","pull_request_target:","workflow_dispatch:"):
 assert bad not in w+t+h,bad
assert "args+=[password]" not in h and 'args+=[username,password]' not in h
print("P8DT_SUPERVISOR_LOCAL_LOGINS_REAL_MQTT_PROOF_SCOPE_CONTRACT_PASS")
