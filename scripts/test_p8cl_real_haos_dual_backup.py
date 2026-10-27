#!/usr/bin/env python3
"""P8cl: actual official HAOS Supervisor TWO disabled YAML automation backup,
private SHA-verified Rust registry+YAML stage and zero-commands preview.

No household VM, Home Assistant config mutation or physical service execution.
"""
import argparse,hashlib,json,subprocess,tempfile
from pathlib import Path
from inspect_ha_backup import inspect_backup

ap=argparse.ArgumentParser()
ap.add_argument("--archive",type=Path,required=True)
ap.add_argument("--manifest",type=Path,required=True)
ap.add_argument("--stager",type=Path,required=True)
ap.add_argument("--native",type=Path,required=True)
a=ap.parse_args()
root=Path(__file__).resolve().parent.parent
archive=a.archive.resolve(strict=True)
manifest=json.loads(a.manifest.read_text())
assert archive.stat().st_size==manifest["size_bytes"]
with archive.open("rb") as f: sha=hashlib.file_digest(f,"sha256").hexdigest()
assert sha==manifest["sha256"]
assert len(manifest["source_boot_id"])==36
assert len(manifest["core_version"])>3
pre=inspect_backup(archive)
assert pre["homeassistant_payload_identified"] is True
assert pre["protected_metadata_flag"] is False
assert not pre["blocking_issues"],pre["blocking_issues"]
fixture=(root/"scripts/fixtures/p8cl_two_inactive_haos_automations.yaml").read_bytes()
assert fixture.count(b"  initial_state: false")==2
expected_yaml_sha=hashlib.sha256(fixture).hexdigest()
runtime=root/"runtime"
runtime.mkdir(mode=0o700,exist_ok=True)
assert not runtime.is_symlink() and runtime.stat().st_mode & 0o077==0
def execute(binary,*args,expect_ok=True):
    p=subprocess.run([str(binary),*map(str,args)],cwd=root,capture_output=True,
                     text=True,timeout=55)
    if expect_ok:
        assert p.returncode==0,("native Rust operation failed",p.stderr[-500:])
        return json.loads(p.stdout)
    assert p.returncode!=0,"unsafe native operation unexpectedly succeeded"
with tempfile.TemporaryDirectory(prefix="p8cl-actual-haos-dual-",dir=runtime) as temp:
    store=Path(temp)
    assert not store.is_symlink() and store.stat().st_mode & 0o077==0
    imported=execute(a.stager,"--lab-readonly",store,"import-unprotected-backup",archive)
    assert imported["archive_sha256"]==sha
    assert imported["automation_source_sha256"]==expected_yaml_sha
    assert imported["automation_source_bytes"]==len(fixture)
    assert imported["opaque_automations_inactive"] is True
    assert imported["entities"]==6 and imported["stable_identities"]==6
    assert imported["read_only"] is True and imported["production_ready"] is False
    assert imported["apps_migrated"] is False and imported["integrations_migrated"] is False
    desc=execute(a.stager,"--lab-readonly",store,"automation-source-status",sha)
    assert desc["source"]["automation_sha256"]==expected_yaml_sha
    assert desc["source"]["registry_sha256"]==imported["sha256"]
    assert desc["source"]["inactive"] is True
    assert desc["actions_executable"] is False and desc["content_returned"] is False
    execute(a.native,"--lab-ha-backup-automations-preview",store,sha,expect_ok=False)
    execute(a.stager,"--lab-readonly",store,"activate",imported["sha256"],"none")
    preview=execute(a.native,"--lab-ha-backup-automations-preview",store,sha)
    assert preview["source"]=="private_sha_bound_haos_backup_automation_preview"
    assert preview["archive_sha256"]==sha
    assert preview["yaml_sha256"]==expected_yaml_sha
    assert preview["registry_sha256"]==imported["sha256"]
    assert preview["explicitly_disabled_count"]==2
    assert [p["source_entity"] for p in preview["plans"]]==[
        "binary_sensor.lab_motion","binary_sensor.lab_door"]
    assert [p["proposed_target"] for p in preview["plans"]]==[
        "light.lab_room","light.lab_entry"]
    assert all(p["proposed_service"]=="light.turn_on" for p in preview["plans"])
    assert preview["read_only"] is True and preview["approval_granted"] is False
    assert preview["device_commands"]==0 and preview["actions_executed"]==0
    assert preview["production_ready"] is False
    # The P8bi Moli fake actuator proof must fail with multi-entry HAOS
    # source: it is only approved to execute a SINGLE synthetic fixture.
    execute(a.native,"--lab-ha-backup-automation-moli",store,sha,expect_ok=False)
    execute(a.native,"--lab-ha-backup-automations-preview",store,"f"*64,expect_ok=False)
    print("P8CL_GENUINE_HAOS_SUPERVISOR_TWO_INACTIVE_YAML_RUST_SHA_BOUND_PREVIEW_PASS")
    execute(a.stager,"--lab-readonly",store,"rollback",imported["sha256"])
    execute(a.native,"--lab-ha-backup-automations-preview",store,sha,expect_ok=False)
    after=execute(a.stager,"--lab-readonly",store,"status")
    assert after["active"] is None
    status=execute(a.stager,"--lab-readonly",store,"automation-source-status",sha)
    assert status["source"]["inactive"] is True and status["actions_executable"] is False
    print("P8CL_REAL_HAOS_DUAL_DISABLED_SOURCE_ROLLBACK_PRESERVED_NO_EXEC_PASS")
