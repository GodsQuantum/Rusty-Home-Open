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
preflight=inspect_backup(archive)
assert preflight["protected_metadata_flag"] is False
assert preflight["supervisor_payload_identified"] is True
assert preflight["homeassistant_payload_identified"] is True
assert not preflight["blocking_issues"],preflight["blocking_issues"]

with tempfile.TemporaryDirectory(prefix="hars-p8aa-real-haos-private-") as tmp:
    root=Path(tmp)/"private"; root.mkdir(mode=0o700)
    def invoke(*args):
        res=subprocess.run([str(a.binary),"--lab-readonly",str(root),*map(str,args)],
                           capture_output=True,text=True,timeout=45)
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
