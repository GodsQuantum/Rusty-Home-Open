#!/usr/bin/env python3
"""Read-only native importer conformance. Optional genuine isolated Core registry.
All enclosing HAOS-format archives are synthetic; never call them actual HAOS backup proofs.
"""
import argparse
from io import BytesIO
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile

p=argparse.ArgumentParser()
p.add_argument("--binary",type=Path,required=True)
p.add_argument("--core-registry",type=Path)
a=p.parse_args()
binary=a.binary.resolve(strict=True)

def run(*args,ok=True):
    completed=subprocess.run([str(binary),"--lab-readonly",str(store),*map(str,args)],
                             text=True,capture_output=True,timeout=25)
    if ok:
        assert completed.returncode==0, (args,completed.stderr)
        assert "core.entity_registry" not in completed.stdout
        return json.loads(completed.stdout)
    assert completed.returncode!=0,args
    assert "HA_REGISTRY_STAGING_FAIL_CLOSED" in completed.stderr
    assert not completed.stdout,args

def fixture():
    if a.core_registry:
        return a.core_registry.read_bytes()
    return json.dumps({"key":"core.entity_registry","version":1,"data":{"entities":[
        {"entity_id":"sensor.vendor_alpha","platform":"any_dynamic_vendor",
         "unique_id":"fixture_a","config_entry_id":"fictional-uuid"},
        {"entity_id":"switch.vendor_beta","platform":"another_dynamic_vendor",
         "unique_id":"fixture_b","config_entry_id":"fictional-uuid"}
    ]}}).encode()

def part_file(registry, *, member=".storage/core.entity_registry",evil=None):
    buf=BytesIO()
    with tarfile.open(fileobj=buf,mode="w:gz") as tf:
        for path,data in ((member,registry),):
            e=tarfile.TarInfo("./"+path)
            e.size=len(data)
            if evil=="symlink":
                e.type=tarfile.SYMTYPE;e.linkname="/etc/passwd";e.size=0
            tf.addfile(e,BytesIO(data) if evil!="symlink" else None)
    return buf.getvalue()

def outer(path, raw, *, protected=False, kind="full", extra=()):
    with tarfile.open(path,"w") as tf:
        contents=[("./backup.json",json.dumps({"type":kind,"protected":protected,
            "addons":[],"homeassistant":{"version":"2026.10.2"}}).encode()),
            ("./homeassistant.tar.gz",raw)]
        contents += list(extra)
        for member,data in contents:
            i=tarfile.TarInfo(member)
            i.size=len(data)
            tf.addfile(i,BytesIO(data))

with tempfile.TemporaryDirectory(prefix="p8aa-native-backup-") as tmp:
    root=Path(tmp)
    store=root/"private"
    store.mkdir(mode=0o700)
    src=root/"source.tar"
    raw=fixture()
    plain=part_file(raw)
    outer(src,plain)
    first=run("import-unprotected-backup",src)
    assert first["entities"]>=2,first
    assert first["stable_identities"]>=2,first
    assert first["read_only"] and not first["activated"],first
    assert not first["integrations_migrated"] and not first["production_ready"],first
    assert run("status")["active"] is None
    staged=first["sha256"]
    run("activate",staged,"none")
    again=run("status")
    assert again["active"]==staged and again["entities"]==first["entities"],again
    # Durable reopen, idempotency, rollback after a new process.
    repeated=run("import-unprotected-backup",src)
    assert repeated["sha256"]==staged
    assert run("rollback",staged)["restored"] is None
    assert run("status")["active"] is None
    print("P8AA_BACKUP_NATIVE_DURABLE_STAGE_AND_ROLLBACK_PASS")
    outer(src,b"SecureTar_V3_ENCRYPTED_SYNTHETIC",protected=True)
    run("import-unprotected-backup",src,ok=False)
    outer(src,plain,protected=True)
    run("import-unprotected-backup",src,ok=False)
    outer(src,plain,kind="partial")
    run("import-unprotected-backup",src,ok=False)
    outer(src,plain,extra=[("backup.json",b"{}")])
    run("import-unprotected-backup",src,ok=False)
    outer(src,plain,extra=[("homeassistant.tar",plain)])
    run("import-unprotected-backup",src,ok=False)
    outer(src,part_file(raw,member="../etc/passwd"))
    run("import-unprotected-backup",src,ok=False)
    outer(src,part_file(raw,evil="symlink"))
    run("import-unprotected-backup",src,ok=False)
    outer(src,part_file(b"not a valid registry"))
    run("import-unprotected-backup",src,ok=False)
    outer(src,plain[:-8])  # gzip trailer / CRC deliberately truncated
    run("import-unprotected-backup",src,ok=False)
    outer(src,plain+part_file(raw))  # appended gzip stream after archive terminator
    run("import-unprotected-backup",src,ok=False)
    outer(src,plain)
    link=root/"forged-link.tar"
    link.symlink_to(src)
    run("import-unprotected-backup",link,ok=False)
    assert run("status")["active"] is None
    print("P8AA_BACKUP_UNTRUSTED_ARCHIVES_FAIL_CLOSED_PASS")
    print("P8AA_ONLY_REGISTRY_STAGED_NO_OTHER_CONFIG_NO_HOUSEHOLD_PASS")
