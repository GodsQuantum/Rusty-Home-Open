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

with tempfile.TemporaryDirectory(prefix="hars-p8aa-real-haos-private-") as tmp:
    root=Path(tmp)/"private"; root.mkdir(mode=0o700)
    def invoke(*args):
        res=subprocess.run([str(a.binary),"--lab-readonly",str(root),*map(str,args)],
                           capture_output=True,text=True,timeout=45)
        if res.returncode:
            explain_archive_shape_without_data(archive)
        assert res.returncode==0,res.stderr
        return json.loads(res.stdout)
    staged=invoke("import-unprotected-backup",archive)
    assert staged["archive_sha256"]==manifest["sha256"]
    assert staged["entities"]==2 and staged["stable_identities"]==2
    assert staged["read_only"] and not staged["production_ready"]
    assert not staged["apps_migrated"] and not staged["integrations_migrated"]
    digest=staged["sha256"]
    assert invoke("status")["active"] is None
    invoke("activate",digest,"none")
    assert invoke("status")["active"]==digest
    assert invoke("status")["entities"]==2
    assert invoke("rollback",digest)["restored"] is None
    assert invoke("status")["active"] is None
print("P8AA_REAL_SUPERVISOR_BACKUP_TO_RUST_DURABLE_ROLLBACK_PASS")
print("P8AA_REAL_HAOS_DATA_PRESERVED_WITHOUT_HOUSEHOLD_OR_DEPLOYMENT_PASS")
