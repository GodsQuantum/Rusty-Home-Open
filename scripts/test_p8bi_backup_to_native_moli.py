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
def archive_bytes(automation):
    rows=[
      {"entity_id":"binary_sensor.lab_motion","platform":"fictional_lab",
       "unique_id":"lab-motion","config_entry_id":"fictional_a"},
      {"entity_id":"light.lab_room","platform":"fictional_lab",
       "unique_id":"lab-light","config_entry_id":"fictional_b"},
    ]
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
