#!/usr/bin/env python3
"""Do not mark native MQTT broker running unless a real official Mosquitto
Supervisor archive, Rust restore, MQTT exchange and new OCI are all tested.
"""
from pathlib import Path
r=Path(__file__).resolve().parents[1]
w=(r/'.github/workflows/p8dr-real-mqtt-restored-haos.yml').read_text()
s=(r/'scripts/test_p8dr_real_mosquitto_broker_on_haos_native_data.py').read_text()
for value in (
 "haos_ova-18.3.qcow2.xz",
 "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
 "90fa213c3441e585d5a60387e9244d0ed38efaf8",
 "-snapshot -accel kvm","runs-on: ubuntu-24.04",
 "p8do_official_mosquitto_qemu_pytest.py",
 "test_p8do_real_mosquitto_native_import.py",
 "test_p8dr_real_mosquitto_broker_on_haos_native_data.py",
 "P8DR_REAL_MQTT_BROKER_ON_HAOS_RESTORED_NATIVE_DATA_AND_RETAINED_RESTART_PASS"):
 assert value in w,value
for value in (
 "docker.io/library/eclipse-mosquitto:2.0.22",
 "app-source-ingest","app-source-verify","app-source-stage-inactive",
 "core_mosquitto", "GITHUB_ACTIONS",
 '"--network","none"', '"--read-only"', '"--cap-drop","ALL"',
 '"--userns=keep-id"', '"--security-opt","no-new-privileges"',
 '"--pids-limit"', '"--memory"',
 '"listener 1883 127.0.0.1', '"allow_anonymous true',
 "persistence true", "persistence_location /mosquitto/data/",
 "mosquitto_pub", "mosquitto_sub", "p8dr-mqtt-retained.db",
 "image_id", "second!=first",
 "re.fullmatch(r\"(?:sha256:)?[0-9a-f]{64}\", image_id)",
 "P8DR_REAL_SUPERVISOR_MOSQUITTO_BACKUP_TO_NATIVE_RUST_VERIFIED_DATA_PASS",
 "P8DR_REAL_MQTT_QOS1_PUBLISH_SUBSCRIBE_RETAINED_ON_RESTORED_NATIVE_DATA_PASS",
 "P8DR_REAL_MQTT_RETAINED_MESSAGE_DURABLE_AFTER_NEW_OCI_INSTANCE_PASS",
 "P8DR_ROOTLESS_NO_NETWORK_NO_HOST_PORT_ORIGINAL_HAOS_ARCHIVE_UNCHANGED_PASS"
):
 assert value in s,value
for ban in ("--network=host", "--privileged", "sudo podman"," -p 1883","-p 1883:",
            "pct exec", "qm stop 100", "192.168.", "pull_request_target:", "workflow_dispatch:"):
 assert ban not in w+s,ban
assert "username" not in s.lower() and "password" not in s.lower()
print("P8DR_REAL_HAOS_MOSQUITTO_TO_ROOTLESS_LOOPBACK_MQTT_RETAINED_SCOPE_CONTRACT_PASS")
