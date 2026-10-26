#!/usr/bin/env python3
"""Guard a genuinely different fourth upstream OCI App (Mosquitto), no home broker."""
from pathlib import Path
root=Path(__file__).resolve().parent.parent
s=(root/"scripts/test_p8cb_mosquitto_retained_restore.sh").read_text()
wf=(root/".github/workflows/p8aw-custom-volume-registry.yml").read_text()
for item in (
    '[[ "$(printenv GITHUB_ACTIONS || true)" == true',
    'docker.io/library/eclipse-mosquitto:2.0',
    'docker.io/library/eclipse-mosquitto@sha256:',
    '--network none --read-only --cap-drop ALL',
    'listener 1883 127.0.0.1', 'persistence true',
    'persistence_location /mosquitto/data/',
        'podman volume export --output "$ARCHIVE" "$source_data"',
    'podman volume export --output "$CONFIG_ARCHIVE" "$source_vol"',
    'podman volume import "$restore_data" "$ARCHIVE"',
    '--install-dual-path-lab',
    'podman volume import "$restore_vol" "$CONFIG_ARCHIVE"',
    'grep -Eq \'(^|/)mosquitto.db$\'',
    'sha256sum -c',
    'P8CB_OFFICIAL_MOSQUITTO_STOPPED_PRIVATE_RETAINED_DB_ARCHIVE_SHA_PASS',
    'P8CB_PRIMARY_MQTT_BROKER_ADDED_NEW_RETAINED_AFTER_BACKUP_PASS',
    'P8CB_FOURTH_STATEFUL_APP_INDEPENDENT_RETAINED_RESTORE_POINT_IN_TIME_PASS',
    'P8CB_RESTORED_BROKER_REMOVAL_PRESERVES_ORIGINAL_RETAINED_DATA_PASS',
    'test ! -s "$WORK/unauthorized-newer.txt"',
    'test "$(sub rh-mqtt-persist lab/p8cb/newer)" = after-checkpoint',
):
    assert item in s,item
assert "sudo podman" not in s
assert "192.168." not in s
assert "rh-mqtt-persist" in s and "rh-mqtt-restore" in s
for item in ("scripts/test_p8cb_mosquitto_retained_restore.sh",
             "scripts/test_p8cb_mosquitto_retained_restore_contract.py",
             "bash scripts/test_p8cb_mosquitto_retained_restore.sh"):
    assert item in wf,item
print("P8CB_FOURTH_UPSTREAM_MOSQUITTO_ROOTLESS_OFFLINE_RETAINED_RESTORE_CONTRACT_PASS")
