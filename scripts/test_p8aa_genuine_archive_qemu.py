#!/usr/bin/env python3
"""Host-side native Rust stage of actual disposable HAOS Supervisor backup.
No transfer of archived data into repository, logs, or external network.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
from inspect_ha_backup import inspect_backup
ap=argparse.ArgumentParser()
ap.add_argument("--archive",type=Path,required=True)
ap.add_argument("--manifest",type=Path,required=True)
ap.add_argument("--binary",type=Path,required=True)
a=ap.parse_args()
archive=a.archive.resolve(strict=True)
manifest=json.loads(a.manifest.read_text())
assert archive.stat().st_size==manifest["size_bytes"]
with archive.open("rb") as stream:
    actual_hash=hashlib.file_digest(stream,"sha256").hexdigest()
assert actual_hash==manifest["sha256"]
def explain_archive_shape_without_data(source):
    """Failure-only public CI telemetry: counts and fixed booleans, never names or values."""
    import tarfile
    summary={"core_part_found":False,"nested_files":0,"nested_dirs":0,
             "nested_links":0,"nested_other":0,"official_registry":False,
             "legacy_registry":False,"registry_at_other_depth":False}
    try:
        with tarfile.open(source,mode="r:") as outer:
            for member in outer:
                name=member.name.removeprefix("./")
                if name not in ("homeassistant.tar.gz","homeassistant.tar") or not member.isfile():
                    continue
                summary["core_part_found"]=True
                inner=outer.extractfile(member)
                assert inner is not None
                with tarfile.open(fileobj=inner,mode="r|*") as nested:
                    for pos,item in enumerate(nested):
                        if pos>=50_000:
                            summary["member_cap_exceeded"]=True
                            break
                        if item.isfile():
                            summary["nested_files"]+=1
                        elif item.isdir():
                            summary["nested_dirs"]+=1
                        elif item.issym() or item.islnk():
                            summary["nested_links"]+=1
                        else:
                            summary["nested_other"]+=1
                        name=item.name.removeprefix("./").rstrip("/")
                        if name=="data/.storage/core.entity_registry":
                            summary["official_registry"]=True
                        elif name==".storage/core.entity_registry":
                            summary["legacy_registry"]=True
                        elif name.endswith("/core.entity_registry"):
                            summary["registry_at_other_depth"]=True
    except (OSError,ValueError,tarfile.TarError):
        summary["parse_failed"]=True
    print("P8AA_SANITIZED_ARCHIVE_SHAPE="+json.dumps(summary,sort_keys=True),
          flush=True)

preflight=inspect_backup(archive)
assert preflight["protected_metadata_flag"] is False
# A Supervisor-created FULL archive is not required to include an
# independent supervisor.tar.gz payload: some versions store Supervisor
# recovery metadata in backup.json. Keep every encryption, Core presence,
# nested path and archive integrity check; do not invent this component.
assert preflight["homeassistant_payload_identified"] is True
assert not preflight["blocking_issues"],preflight["blocking_issues"]

runtime=Path(__file__).resolve().parent.parent/"runtime"
runtime.mkdir(mode=0o700,exist_ok=True)
assert not runtime.is_symlink() and runtime.stat().st_mode & 0o077==0
with tempfile.TemporaryDirectory(prefix="hars-p8aa-real-haos-private-",dir=runtime) as tmp:
    # Single private child of repository runtime, satisfying the strict native
    # Moli lab's no-arbitrary-path rule, with automatic disposal.
    root=Path(tmp)
    assert not root.is_symlink() and root.stat().st_mode & 0o077 == 0
    def invoke(*args):
        res=subprocess.run([str(a.binary),"--lab-readonly",str(root),*map(str,args)],
                           capture_output=True,text=True,timeout=45)
        if res.returncode:
            explain_archive_shape_without_data(archive)
        assert res.returncode==0,res.stderr
        return json.loads(res.stdout)
    staged=invoke("import-unprotected-backup",archive)
    # Distinct P8be proof: actual Supervisor must include the exact original
    # source file, Rust must keep it inactive inside its private durable redb.
    automation_fixture=(Path(__file__).resolve().parent/"fixtures"/"p8be_haos_automations.yaml").read_bytes()
    automation_sha=hashlib.sha256(automation_fixture).hexdigest()
    assert staged["opaque_automations_inactive"] is True
    assert staged["automation_source_sha256"]==automation_sha
    assert staged["automation_source_bytes"]==len(automation_fixture)
    detail=invoke("automation-source-status",manifest["sha256"])
    assert detail["source"]["automation_sha256"]==automation_sha
    assert detail["source"]["registry_sha256"]==staged["sha256"]
    assert detail["source"]["inactive"] is True
    assert detail["content_returned"] is False and detail["actions_executable"] is False
    assert staged["archive_sha256"]==manifest["sha256"]
    assert staged["entities"]==4 and staged["stable_identities"]==4
    assert staged["read_only"] and not staged["production_ready"]
    assert not staged["apps_migrated"] and not staged["integrations_migrated"]
    digest=staged["sha256"]
    assert invoke("status")["active"] is None
    invoke("activate",digest,"none")
    assert invoke("status")["active"]==digest
    assert invoke("status")["entities"]==4
    # P8cj: new bounded-list preview API also accepts the exact one-entry
    # source from a REAL official HAOS 18.3 Supervisor full backup. This is
    # only a compatibility smoke for the new API, not a two-automation HAOS
    # migration proof. Separate synthetic two-item archive covers atomicity.
    core_bin=Path(__file__).resolve().parent.parent/"target/debug/hars-native-kernel"
    preview_run=subprocess.run([str(core_bin), "--lab-ha-backup-automations-preview",
                                str(root), manifest["sha256"]],
                               cwd=Path(__file__).resolve().parent.parent,
                               capture_output=True,text=True,timeout=35)
    assert preview_run.returncode==0,preview_run.stderr[-600:]
    preview=json.loads(preview_run.stdout)
    assert preview["source"]=="private_sha_bound_haos_backup_automation_preview"
    assert preview["archive_sha256"]==manifest["sha256"]
    assert preview["yaml_sha256"]==automation_sha
    assert preview["registry_sha256"]==digest
    assert preview["explicitly_disabled_count"]==1 and len(preview["plans"])==1
    assert preview["plans"][0]["source_entity"]=="binary_sensor.lab_motion"
    assert preview["read_only"] and not preview["approval_granted"]
    assert preview["actions_executed"]==preview["device_commands"]==0
    assert preview["production_ready"] is False
    print("P8CJ_REAL_SUPERVISOR_ARCHIVE_SINGLE_INACTIVE_PREVIEW_API_PASS")
    # Distinct P8bi proof: only the original disabled automation from this
    # genuine Supervisor archive may be compiled, approved for invented Moli
    # devices and executed with *no* HAOS real service/hardware access.
    core_bin=Path(__file__).resolve().parent.parent/"target/debug/hars-native-kernel"
    parsed=subprocess.run([str(core_bin), "--lab-ha-backup-automation-moli",
                            str(root),manifest["sha256"]],
                          cwd=Path(__file__).resolve().parent.parent,
                          capture_output=True,text=True,timeout=35)
    assert parsed.returncode==0, parsed.stderr[-600:]
    native=json.loads(parsed.stdout)
    assert native["archive_sha256"]==manifest["sha256"]
    assert native["original_yaml_sha256"]==automation_sha
    assert native["original_registry_sha256"]==digest
    assert native["one_explicitly_disabled_automation"]
    assert native["compiled_to_native_moli"] and native["synthetic_actuator_confirmed"]
    assert native["approval_required"] and native["edit_revokes_approval"]
    assert not native["real_hardware_touched"] and not native["production_ready"]
    print("P8BI_REAL_HAOS_BACKUP_DISABLED_AUTOMATION_ACTUAL_SYNTHETIC_MOLI_GRAPH_PASS")
    assert invoke("rollback",digest)["restored"] is None
    assert invoke("status")["active"] is None
    assert invoke("automation-source-status",manifest["sha256"])["source"]["automation_sha256"]==automation_sha
    print("P8BE_REAL_HAOS_SUPERVISOR_AUTOMATIONS_BYTE_EXACT_INACTIVE_PERSISTENCE_PASS")
print("P8AA_REAL_SUPERVISOR_BACKUP_TO_RUST_DURABLE_ROLLBACK_PASS")
print("P8AA_REAL_HAOS_DATA_PRESERVED_WITHOUT_HOUSEHOLD_OR_DEPLOYMENT_PASS")
