"""P8cl new real Supervisor dual-automation snapshot (no actual devices).
The registry rows are deliberately synthetic and inserted into a disposable
guest; the outer+inner archive MUST be created by real Supervisor in QEMU.
No household Home Assistant, device or live credentials.
"""
import base64
import hashlib
from pathlib import Path
import os
import json
import pytest
import re

def write_guest_bytes_over_serial(shell, data, destination, staging):
    """Bounded single-line console commands: labgrid/HAOS serial truncates long input."""
    encoded = base64.b64encode(data).decode("ascii")
    shell.run_check(": > " + staging)
    for offset in range(0, len(encoded), 96):
        chunk = encoded[offset:offset + 96]
        shell.run_check("printf %s " + chunk + " >> " + staging)
    shell.run_check("base64 -d " + staging + " > " + destination + " && rm -f " + staging)

@pytest.mark.dependency(depends=["test_start_supervisor"])
@pytest.mark.timeout(900)
def test_hars_dual_inactive_automation_source(shell,shell_json):
    fake=json.dumps({"key":"core.entity_registry","version":1,"minor_version":1,
      "data":{"entities":[
        {"entity_id":"sensor.fixture_vendor_one","platform":"fictional_vendor_1",
         "unique_id":"fictional_serial_a","config_entry_id":"fictional_config_1"},
        {"entity_id":"switch.fixture_vendor_two","platform":"fictional_vendor_2",
         "unique_id":"fictional_serial_b","config_entry_id":"fictional_config_2"},
        {"entity_id":"binary_sensor.lab_motion","platform":"fictional_lab",
         "unique_id":"fictional_lab_motion_serial","config_entry_id":"fictional_config_3"},
        {"entity_id":"light.lab_room","platform":"fictional_lab",
         "unique_id":"fictional_lab_light_serial","config_entry_id":"fictional_config_4"},
        {"entity_id":"binary_sensor.lab_door","platform":"fictional_lab",
         "unique_id":"fictional_lab_door_serial","config_entry_id":"fictional_config_5"},
        {"entity_id":"light.lab_entry","platform":"fictional_lab",
         "unique_id":"fictional_lab_entry_serial","config_entry_id":"fictional_config_6"}]}},
        separators=(",",":"),sort_keys=True).encode()
    home="/mnt/data/supervisor/homeassistant/.storage"
    source=home+"/core.entity_registry"
    shell.run_check("mkdir -p "+home)
    write_guest_bytes_over_serial(shell, fake, source, "/tmp/p8cl-registry.b64")
    # A new DISTINCT fixture alongside the prior 2-entity Core identity test.
    # The file is inert and never executed by the disposable Core.
    fixture=Path(os.environ["GITHUB_WORKSPACE"])/"scripts/fixtures/p8cl_two_inactive_haos_automations.yaml"
    automation=fixture.read_bytes()
    assert automation.count(b"  initial_state: false")==2
    assert b"p8cl_second_disabled_lab_door" in automation
    assert b"initial_state: true" not in automation
    automation_path="/mnt/data/supervisor/homeassistant/automations.yaml"
    write_guest_bytes_over_serial(shell, automation, automation_path, "/tmp/p8cl-automations.b64")
    assert shell.run_check("sha256sum "+automation_path)[0].split()[0] == hashlib.sha256(automation).hexdigest()
    result=shell.run_check("sha256sum "+source)
    assert len(result)==1
    assert result[0].split()[0]==hashlib.sha256(fake).hexdigest()
    boot=shell.run_check("cat /proc/sys/kernel/random/boot_id")[0].strip()
    assert re.fullmatch(r"[0-9a-f-]{36}",boot)
    core=shell_json("ha core info --no-progress --raw-json",timeout=90)
    version=(core.get("data") or {}).get("version")
    assert core.get("result")=="ok" and re.fullmatch(r"[0-9A-Za-z_.+-]{1,64}",version)
    created=shell_json('ha backups new --name "HARS Two Inactive Synthetic Automations" --no-progress --raw-json',timeout=390)
    slug=(created.get("data") or {}).get("slug")
    assert created.get("result")=="ok" and re.fullmatch(r"[0-9a-f]{8,16}",slug)
    info=shell_json("ha backups info "+slug+" --no-progress --raw-json",timeout=100)
    row=info.get("data") or {}
    assert info.get("result")=="ok" and row.get("type")=="full"
    assert row.get("slug")==slug and row.get("homeassistant")==version
    assert row.get("protected") is False, "source must be unencrypted for Rust read-only importer"
    tar="/mnt/data/supervisor/backup/"+slug+".tar"
    assert shell.run_check("test -s "+tar+" && echo PRESENT")[0].strip()=="PRESENT"
    transferred=shell.run_check(
        "curl --fail --silent --show-error --max-time 450 "+
        "-H 'X-Hars-Slug: "+slug+"' -H 'X-Hars-Boot: "+boot+"' "+
        "-H 'X-Hars-Core: "+version+"' -T "+tar+
        " http://10.0.2.2:18674/archive && echo P8AA_EXPORT_DONE",timeout=480)
    assert "P8AA_EXPORT_DONE" in transferred
    print("P8CL_REAL_HAOS_SUPERVISOR_DUAL_DISABLED_ARCHIVE_EXPORTED_PASS")
    print("P8CL_TWO_INACTIVE_YAML_BYTES_INSIDE_ACTUAL_SUPERVISOR_BACKUP_PASS")
