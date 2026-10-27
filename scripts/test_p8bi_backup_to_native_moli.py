#!/usr/bin/env python3
"""Fail-closed synthetic unit bridge before the separate real HAOS QEMU test.

Exercise byte-exact native custody -> explicitly disabled one-YAML subset ->
actual upstream Moli approval/fake driver. No real HAOS or hardware used here.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile

parser=argparse.ArgumentParser()
parser.add_argument("--stager", type=Path, required=True)
parser.add_argument("--native", type=Path, required=True)
arg=parser.parse_args()
ROOT=Path(__file__).resolve().parent.parent
source=(ROOT/"scripts/fixtures/p8be_haos_automations.yaml").read_bytes()
def archive_bytes(automation, *, extra_lab_entities=False):
    rows=[
      {"entity_id":"binary_sensor.lab_motion","platform":"fictional_lab",
       "unique_id":"lab-motion","config_entry_id":"fictional_a"},
      {"entity_id":"light.lab_room","platform":"fictional_lab",
       "unique_id":"lab-light","config_entry_id":"fictional_b"},
    ]
    if extra_lab_entities:
        rows.extend([
          {"entity_id":"binary_sensor.lab_door","platform":"fictional_lab",
           "unique_id":"lab-door","config_entry_id":"fictional_c"},
          {"entity_id":"light.lab_entry","platform":"fictional_lab",
           "unique_id":"lab-entry","config_entry_id":"fictional_d"},
        ])
    registry=json.dumps({"key":"core.entity_registry","version":1,
                         "data":{"entities":rows}}).encode()
    buf=io.BytesIO()
    with tarfile.open(fileobj=buf,mode="w:gz") as inner:
        for name,body in [("data/.storage/core.entity_registry",registry),
                          ("data/automations.yaml",automation)]:
            info=tarfile.TarInfo(name);info.size=len(body)
            inner.addfile(info,io.BytesIO(body))
    tar_core=buf.getvalue()
    out=io.BytesIO()
    with tarfile.open(fileobj=out,mode="w") as outer:
        for name,body in [("backup.json",b'{"type":"full","protected":false}'),
                          ("homeassistant.tar.gz",tar_core)]:
            info=tarfile.TarInfo(name);info.size=len(body)
            outer.addfile(info,io.BytesIO(body))
    return out.getvalue()
runtime=ROOT/"runtime"
runtime.mkdir(mode=0o700,exist_ok=True)
assert not runtime.is_symlink() and runtime.stat().st_mode & 0o077==0
with tempfile.TemporaryDirectory(prefix="p8bi-",dir=runtime) as tmp:
    directory=Path(tmp)
    full=directory/"synthetic.tar"
    full.write_bytes(archive_bytes(source))
    digest=hashlib.sha256(full.read_bytes()).hexdigest()
    def call(binary,*argv,ok=True):
        p=subprocess.run([str(binary),*map(str,argv)],cwd=ROOT,capture_output=True,text=True,timeout=35)
        if ok:
            assert p.returncode==0,p.stderr[-600:]
            return json.loads(p.stdout)
        assert p.returncode!=0,p.stdout
    first=call(arg.stager,"--lab-readonly",directory,"import-unprotected-backup",full)
    assert first["archive_sha256"]==digest
    assert first["automation_source_sha256"]==hashlib.sha256(source).hexdigest()
    assert first["entities"]==2
    call(arg.native,"--lab-ha-backup-automation-moli",directory,digest,ok=False)
    call(arg.stager,"--lab-readonly",directory,"activate",first["sha256"],"none")
    proof=call(arg.native,"--lab-ha-backup-automation-moli",directory,digest)
    assert proof["archive_sha256"]==digest
    assert proof["original_yaml_sha256"]==hashlib.sha256(source).hexdigest()
    assert proof["original_registry_sha256"]==first["sha256"]
    assert proof["one_explicitly_disabled_automation"]
    assert proof["compiled_to_native_moli"]
    assert proof["synthetic_actuator_confirmed"] and proof["approval_required"]
    assert proof["edit_revokes_approval"] and not proof["real_hardware_touched"]
    assert proof["persisted_edit_inactive"] and proof["stale_approval_rejected"]
    assert proof["no_run_after_edit"] and proof["restart_kept_edit_inactive"]
    assert proof["production_ready"] is False
    call(arg.stager,"--lab-readonly",directory,"rollback",first["sha256"])
    call(arg.native,"--lab-ha-backup-automation-moli",directory,digest,ok=False)
    call(arg.native,"--lab-ha-backup-automation-moli",directory,"f"*64,ok=False)
    print("P8BI_SYNTHETIC_ARCHIVE_BOUND_DISABLED_YAML_ACTUAL_MOLI_APPROVAL_AND_ROLLBACK_PASS")

# NEW distinct P8cj: genuine native offline archive importer -> redb SHA
# custody -> MULTI-automation disabled-only atomic preview, no Moli execution.
second=source.decode().replace(
  "p8bi_official_archive_disabled_lab_motion","p8cj_second_lab_door"
).replace("binary_sensor.lab_motion","binary_sensor.lab_door"
).replace("light.lab_room","light.lab_entry")
both=(source.decode().rstrip()+"\n"+second).encode()
with tempfile.TemporaryDirectory(prefix="p8cj-",dir=runtime) as temp:
    folder=Path(temp)
    archive=folder/"disabled-pair.tar"
    archive.write_bytes(archive_bytes(both,extra_lab_entities=True))
    sha=hashlib.sha256(archive.read_bytes()).hexdigest()
    imported=call(arg.stager,"--lab-readonly",folder,"import-unprotected-backup",archive)
    assert imported["automation_source_sha256"]==hashlib.sha256(both).hexdigest()
    # NO preview before the original identity registry is safely active.
    call(arg.native,"--lab-ha-backup-automations-preview",folder,sha,ok=False)
    call(arg.stager,"--lab-readonly",folder,"activate",imported["sha256"],"none")
    preview=call(arg.native,"--lab-ha-backup-automations-preview",folder,sha)
    assert preview["archive_sha256"]==sha
    assert preview["yaml_sha256"]==hashlib.sha256(both).hexdigest()
    assert preview["registry_sha256"]==imported["sha256"]
    assert preview["explicitly_disabled_count"]==2
    assert [p["source_entity"] for p in preview["plans"]]==[
        "binary_sensor.lab_motion","binary_sensor.lab_door"]
    assert [p["proposed_target"] for p in preview["plans"]]==[
        "light.lab_room","light.lab_entry"]
    assert preview["read_only"] is True
    assert preview["approval_granted"] is False
    assert preview["actions_executed"]==preview["device_commands"]==0
    assert preview["production_ready"] is False
    # Single-automation executable sandbox MUST reject the multi-list.
    call(arg.native,"--lab-ha-backup-automation-moli",folder,sha,ok=False)
    call(arg.native,"--lab-ha-backup-automations-preview",folder,"f"*64,ok=False)
    call(arg.stager,"--lab-readonly",folder,"rollback",imported["sha256"])
    call(arg.native,"--lab-ha-backup-automations-preview",folder,sha,ok=False)
    print("P8CJ_SHA_BOUND_TWO_DISABLED_AUTOMATIONS_ATOMIC_PREVIEW_WITH_NO_DEVICE_COMMANDS_PASS")

# A second archive can be checksum-valid yet contain a malicious member;
# importing the bytes for custody MUST NOT grant permission to preview/use it.
unsafe=(source.decode().rstrip()+"\n"+second.replace(
    "  initial_state: false", "  initial_state: true", 1)).encode()
with tempfile.TemporaryDirectory(prefix="p8cj-bad-",dir=runtime) as temp:
    folder=Path(temp)
    archive=folder/"hostile-pair.tar"
    archive.write_bytes(archive_bytes(unsafe,extra_lab_entities=True))
    sha=hashlib.sha256(archive.read_bytes()).hexdigest()
    imported=call(arg.stager,"--lab-readonly",folder,"import-unprotected-backup",archive)
    call(arg.stager,"--lab-readonly",folder,"activate",imported["sha256"],"none")
    call(arg.native,"--lab-ha-backup-automations-preview",folder,sha,ok=False)
print("P8CJ_CHECKSUM_VALID_BUT_UNSAFE_AUTOMATION_BATCH_DENIED_PASS")
