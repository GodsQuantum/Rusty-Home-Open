"""Two independent HAOS boots, two separate upstream QEMU -snapshot runs.

Source and target run via separate invocations of upstream run_tests.sh.
Only a bounded SHA256-attested synthetic archive crosses the host relay.
"""
import pytest
import re
from time import sleep

URL="http://10.0.2.2:18674"
MARK="/mnt/data/supervisor/share/rusty_home_fresh_vm_marker"
VALUE="P8P_FRESH_SECOND_HAOS_VERIFIED_SOURCE_20261024"
def one(shell,command,timeout=45):
    result=shell.run_check(command,timeout=timeout)
    assert len(result)==1,(command,len(result))
    return result[0].strip()

@pytest.mark.dependency(depends=["test_start_supervisor"])
@pytest.mark.timeout(900)
def test_hars_cross_vm_source(shell,shell_json):
    shell.run_check("mkdir -p /mnt/data/supervisor/share")
    shell.run_check("sh -c 'echo "+VALUE+" > "+MARK+"'")
    assert one(shell,"cat "+MARK)==VALUE
    boot=one(shell,"cat /proc/sys/kernel/random/boot_id")
    assert re.fullmatch(r"[0-9a-f-]{36}",boot)
    core=shell_json("ha core info --no-progress --raw-json",timeout=90)
    version=(core.get("data") or {}).get("version")
    assert core.get("result")=="ok" and isinstance(version,str)
    assert re.fullmatch(r"[a-zA-Z0-9_.+-]{1,64}",version)
    created=shell_json('ha backups new --name "HARS Fresh VM Fixture" --no-progress --raw-json',timeout=390)
    slug=(created.get("data") or {}).get("slug")
    assert created.get("result")=="ok" and isinstance(slug,str) and re.fullmatch(r"[0-9a-f]{8,16}",slug)
    info=shell_json("ha backups info "+slug+" --no-progress --raw-json",timeout=100)
    row=info.get("data") or {}
    assert info.get("result")=="ok" and row.get("type")=="full"
    assert row.get("slug")==slug and row.get("homeassistant")==version
    assert "share" in row.get("folders",[])
    tar="/mnt/data/supervisor/backup/"+slug+".tar"
    assert one(shell,"test -s "+tar+" && echo PRESENT")=="PRESENT"
    transferred=shell.run_check(
        "curl --fail --silent --show-error --max-time 450 "
        "-H 'X-Hars-Slug: "+slug+"' -H 'X-Hars-Boot: "+boot+"' "
        "-H 'X-Hars-Core: "+version+"' -T "+tar+" "+URL+"/archive "
        "&& echo P8P_EXPORT_DONE", timeout=480)
    assert "P8P_EXPORT_DONE" in transferred
    print("P8P_REAL_SOURCE_FULL_BACKUP_EXPORTED_PASS")

@pytest.mark.dependency(depends=["test_start_supervisor"])
@pytest.mark.timeout(1200)
def test_hars_cross_vm_target(shell,shell_json):
    assert one(shell,"test ! -e "+MARK+" && echo CLEAN")=="CLEAN"
    target_boot=one(shell,"cat /proc/sys/kernel/random/boot_id")
    shell.run_check("curl --fail --silent --show-error --max-time 30 "+URL+
                    "/manifest -o /tmp/p8p-manifest.json",timeout=50)
    def field(name):
        return one(shell,"jq -er '."+name+"' /tmp/p8p-manifest.json")
    slug=field("slug")
    boot=field("source_boot_id")
    digest=field("sha256")
    version=field("core_version")
    size=field("size_bytes")
    assert re.fullmatch(r"[0-9a-f]{8,16}",slug)
    assert re.fullmatch(r"[0-9a-f-]{36}",boot) and boot!=target_boot
    assert re.fullmatch(r"[0-9a-f]{64}",digest)
    assert size.isdigit() and 1024<int(size)<=1024*1024*1024
    assert re.fullmatch(r"[a-zA-Z0-9_.+-]{1,64}",version)
    tar="/mnt/data/supervisor/backup/"+slug+".tar"
    shell.run_check("mkdir -p /mnt/data/supervisor/backup")
    shell.run_check("curl --fail --silent --show-error --max-time 450 "+
                    URL+"/archive -o "+tar,timeout=480)
    assert one(shell,"stat -c %s "+tar)==size
    assert one(shell,"sha256sum "+tar).split()[0]==digest
    loaded=shell_json("ha backups reload --no-progress --raw-json",timeout=120)
    assert loaded.get("result")=="ok"
    details=shell_json("ha backups info "+slug+" --no-progress --raw-json",timeout=100)
    row=details.get("data") or {}
    assert details.get("result")=="ok" and row.get("slug")==slug
    assert row.get("type")=="full" and row.get("homeassistant")==version
    assert "share" in row.get("folders",[])
    assert one(shell,"test ! -e "+MARK+" && echo CLEAN")=="CLEAN"
    done=shell_json("ha backups restore "+slug+" --no-progress --raw-json",timeout=540)
    assert done.get("result")=="ok"
    recovered=False
    for _ in range(60):
        try:
            if one(shell,"cat "+MARK)==VALUE:
                core=shell_json("ha core info --no-progress --raw-json",timeout=45)
                sup=shell_json("ha supervisor info --no-progress --raw-json",timeout=45)
                if (core.get("result")=="ok" and sup.get("result")=="ok" and
                    (core.get("data") or {}).get("version")==version):
                    recovered=True
                    break
        except Exception:
            pass
        sleep(5)
    assert recovered,"Independent fresh target did not restore source data and Core"
    assert one(shell,"cat /proc/sys/kernel/random/boot_id")==target_boot
    print("P8P_GENUINE_SECOND_FRESH_HAOS_FULL_RESTORE_PASS")
