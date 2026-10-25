"""New P8aa source-only HAOS certification scenario.
The registry rows are deliberately synthetic and inserted into a disposable
guest; the outer+inner archive MUST be created by real Supervisor in QEMU.
No household Home Assistant, device or live credentials.
"""
import base64
import hashlib
import json
import pytest
import re

@pytest.mark.dependency(depends=["test_start_supervisor"])
@pytest.mark.timeout(900)
def test_hars_native_backup_source(shell,shell_json):
    fake=json.dumps({"key":"core.entity_registry","version":1,"minor_version":1,
      "data":{"entities":[
        {"entity_id":"sensor.fixture_vendor_one","platform":"fictional_vendor_1",
         "unique_id":"fictional_serial_a","config_entry_id":"fictional_config_1"},
        {"entity_id":"switch.fixture_vendor_two","platform":"fictional_vendor_2",
         "unique_id":"fictional_serial_b","config_entry_id":"fictional_config_2"}]}},
        separators=(",",":"),sort_keys=True).encode()
    encoded=base64.b64encode(fake).decode("ascii")
    home="/mnt/data/supervisor/homeassistant/.storage"
    source=home+"/core.entity_registry"
    shell.run_check("mkdir -p "+home)
    shell.run_check("sh -c 'printf %s "+encoded+" | base64 -d > "+source+"'")
    result=shell.run_check("sha256sum "+source)
    assert len(result)==1
    assert result[0].split()[0]==hashlib.sha256(fake).hexdigest()
    boot=shell.run_check("cat /proc/sys/kernel/random/boot_id")[0].strip()
    assert re.fullmatch(r"[0-9a-f-]{36}",boot)
    core=shell_json("ha core info --no-progress --raw-json",timeout=90)
    version=(core.get("data") or {}).get("version")
    assert core.get("result")=="ok" and re.fullmatch(r"[0-9A-Za-z_.+-]{1,64}",version)
    created=shell_json('ha backups new --name "HARS Native Source Fixture" --no-progress --raw-json',timeout=390)
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
    print("P8AA_REAL_HAOS_SUPERVISOR_CREATED_BACKUP_EXPORTED_PASS")
