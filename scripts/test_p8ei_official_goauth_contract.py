#!/usr/bin/env python3
"""P8ei exact offline lab boundary, fail closed for permissive Docker configs."""
from pathlib import Path
import re
root=Path(__file__).resolve().parents[1]
src=(root/"crates/mqtt-supervisor-auth-lab/src/bin/p8ei_actual_mqtt_lab.rs").read_text()
runner=(root/"scripts/test_p8ei_official_go_auth_real_mqtt.py").read_text()
workflow=(root/".github/workflows/p8ei-official-mosquitto-actual-goauth.yml").read_text()
required_src=[
    "MqttSupervisorAuthLab", "LabAuthApiPermission::FixtureGranted",
    'FAKE_TOKEN', 'http_reply', "TcpListener::bind(addr(GATE))",
    "TcpListener::bind(addr(SUP))", "connect_probe", "response[3] == 0",
    "probe-denied-valid", "P8EI_OFFICIAL_MOSQUITTO_GO_AUTH_MQTT_"
]
# The literal constant token is deliberately fictional and never loaded from
# a user's account or environment.
for text in required_src:
    if text=="FAKE_TOKEN": continue
    assert text in src,text
assert "P8EI_SYNTHETIC_SUPERVISOR_API_TEST_TOKEN" in src
for text in [
    "homeassistant/amd64-addon-mosquitto@sha256:",
    "a52dd07cb946a37837864b681705834bd06fb15068a14155537504cf54271e5e",
    'PLUGIN = "/usr/share/mosquitto/go-auth.so"',
    '"--network","none"', '"--read-only"', '"--cap-drop","ALL"',
    '"--security-opt","no-new-privileges"', '"--user","1000:1000"',
    '"--tmpfs","/tmp:rw,nosuid,nodev,mode=1777"',
    'host["NetworkMode"]=="none"', 'host["ReadonlyRootfs"] is True',
    'host["PortBindings"] in (None, {})',
    "auth_opt_backends http", "auth_opt_cache false",
    "auth_opt_http_getuser_uri /authentication",
    "auth_opt_http_superuser_uri /superuser",
    "auth_opt_http_aclcheck_uri /acl", "allow_anonymous false",
    "control.write_text(\"deny\\n\")",
    "control.write_text(\"outage\\n\")",
    "control.write_text(\"permit\\n\")",
    'run(["docker","rm","-f",name]', "P8EI_CONTAINER_FULL_CLEANUP_NO_HOST_PORTS_PASS",
]:
    assert text in runner,text
for forbidden in [
    "--privileged", '--network","host"', 'ports:', '"-p",',
    '192.168.', 'SUPERVISOR_TOKEN', 'qm stop', 'pct exec',
    'workflow_dispatch:', 'pull_request_target:', 'secrets.'
]:
    assert forbidden not in src+runner+workflow,forbidden
assert "docker pull" not in workflow or "--network" in runner
assert 'crates/mqtt-supervisor-auth-lab' in workflow
assert 'crates/supervisor-apps' not in workflow
print("P8EI_OFFICIAL_IMAGE_ACTUAL_GOAUTH_MQTT_SCOPE_CONTRACT_PASS")
