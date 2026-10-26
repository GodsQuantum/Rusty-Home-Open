#!/usr/bin/env python3
"""Synthetic HAOS-format archive only, proves private, reversible YAML custody.
No actual HAOS backup, household data or executable YAML is used.
"""
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import argparse

ap=argparse.ArgumentParser()
ap.add_argument("--binary",required=True,type=Path)
args=ap.parse_args()
binary=args.binary.resolve(strict=True)
private_yaml=b"- id: fixture_1\n  alias: 'Private {{ do_not_interpret }}'\n  trigger: []\n  action: []\n"
registry=json.dumps({"key":"core.entity_registry","version":1,"data":{"entities":[
    {"entity_id":"binary_sensor.lab_motion","platform":"fake","unique_id":"fixture-123","config_entry_id":"fake-entry"}
]}}).encode()

def make_tar(rows, compressed=False):
    buf=io.BytesIO()
    with tarfile.open(fileobj=buf,mode="w:gz" if compressed else "w") as tf:
        for name, content in rows:
            item=tarfile.TarInfo("./"+name)
            item.size=len(content)
            tf.addfile(item,io.BytesIO(content))
    return buf.getvalue()

def archive(out, yaml=private_yaml, path="data/automations.yaml", extra=()):
    payload=make_tar([("data/.storage/core.entity_registry",registry),(path,yaml),*extra],compressed=True)
    outer=make_tar([("backup.json",json.dumps({"type":"full","protected":False}).encode()),
                    ("homeassistant.tar.gz",payload)])
    out.write_bytes(outer)
    return hashlib.sha256(outer).hexdigest()

with tempfile.TemporaryDirectory(prefix="p8be-fake-automation-") as tmp:
    base=Path(tmp)
    private=base/"private";private.mkdir(mode=0o700)
    source=base/"full.tar"
    expected_archive=archive(source)
    expected_yaml=hashlib.sha256(private_yaml).hexdigest()
    def call(*argv,good=True):
        p=subprocess.run([str(binary),"--lab-readonly",str(private),*map(str,argv)],
                         text=True,capture_output=True,timeout=30)
        if not good:
            assert p.returncode!=0 and not p.stdout and "HA_REGISTRY_STAGING_FAIL_CLOSED" in p.stderr,repr(argv)
            return None
        assert p.returncode==0,(argv,p.stderr)
        assert "do_not_interpret" not in p.stdout
        return json.loads(p.stdout)
    before=call("status")
    assert before["active"] is None
    staged=call("import-unprotected-backup",source)
    assert staged["archive_sha256"]==expected_archive
    assert staged["automation_source_sha256"]==expected_yaml
    assert staged["automation_source_bytes"]==len(private_yaml)
    assert staged["opaque_automations_inactive"] is True
    assert staged["automations_migrated"] is False
    audit=call("automation-source-status",expected_archive)
    assert audit["source"]["automation_sha256"]==expected_yaml
    assert audit["source"]["registry_sha256"]==staged["sha256"]
    assert audit["source"]["inactive"] is True
    assert not audit["content_returned"] and not audit["actions_executable"]
    assert call("automation-source-status","0"*64)["source"] is None
    assert call("status")["active"] is None
    assert (private/"registry-shadow.redb").stat().st_mode & 0o077==0
    call("activate",staged["sha256"],"none")
    assert call("status")["active"]==staged["sha256"]
    call("rollback",staged["sha256"])
    assert call("status")["active"] is None
    assert call("automation-source-status",expected_archive)["source"]["automation_sha256"]==expected_yaml
    assert call("import-unprotected-backup",source)["sha256"]==staged["sha256"]
    print("P8BE_AUTOMATION_SOURCE_BYTE_EXACT_DURABLE_READ_ONLY_STAGE_ROLLBACK_PASS")
    archive(source,path="automations.yaml")
    assert call("import-unprotected-backup",source)["automation_source_sha256"]==expected_yaml
    archive(source,extra=[("automations.yaml",private_yaml)])
    call("import-unprotected-backup",source,good=False)
    archive(source,extra=[("data/automations.yaml",b"duplicate")])
    call("import-unprotected-backup",source,good=False)
    archive(source,yaml=b"\xff\xfe")
    call("import-unprotected-backup",source,good=False)
    archive(source,yaml=b"X"*(4*1024*1024+1))
    call("import-unprotected-backup",source,good=False)
    archive(source,extra=[("../escape",b"x")])
    call("import-unprotected-backup",source,good=False)
    assert call("status")["active"] is None
    assert call("automation-source-status",expected_archive)["source"]["automation_sha256"]==expected_yaml
    print("P8BE_AMBIGUOUS_UNSAFE_OVERSIZED_AUTOMATION_SOURCES_FAIL_CLOSED_PASS")
