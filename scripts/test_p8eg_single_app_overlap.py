#!/usr/bin/env python3
"""P8eg: real native Rust single-App staging must protect immutable source escrow.

No home devices, real disks, actual backups, network, or external account.
Fictional archive is an official-shaped App-only tar for a new isolated case.
"""
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile

binary = str(Path(sys.argv[1]).resolve(strict=True))

def archive(path):
    data = b"P8EG_NEW_SYNTHETIC_SINGLE_APP_20261028\\n"
    body = io.BytesIO()
    with tarfile.open(fileobj=body, mode="w:gz") as bundle:
        entry = tarfile.TarInfo("./data/p8eg-private-marker.txt")
        entry.size = len(data)
        bundle.addfile(entry, io.BytesIO(data))
    with tarfile.open(path, mode="w") as outer:
        for name, content in (
            ("./backup.json", json.dumps({"type":"partial", "protected":False,
                                          "addons":["core_configurator"]}).encode()),
            ("./core_configurator.tar.gz", body.getvalue()),
        ):
            entry = tarfile.TarInfo(name)
            entry.size = len(content)
            outer.addfile(entry, io.BytesIO(content))
    path.chmod(0o600)
    return hashlib.sha256(path.read_bytes()).hexdigest(), data

def call(escrow, *args):
    return subprocess.run([binary, "--lab-readonly", str(escrow), *map(str,args)],
                          capture_output=True, text=True, timeout=45)

with tempfile.TemporaryDirectory(prefix="p8eg-single-overlap-") as scratch:
    root = Path(scratch)
    escrow = root/"escrow"; escrow.mkdir(mode=0o700)
    disjoint = root/"output"; disjoint.mkdir(mode=0o700)
    descendant = escrow/"nested-output"; descendant.mkdir(mode=0o700)
    source=root/"source-fixture.tar"
    digest, marker = archive(source)
    ingested=call(escrow, "app-source-ingest", source)
    assert ingested.returncode==0, ingested.stderr
    assert json.loads(ingested.stdout)["sha256"]==digest
    immutable=escrow/(digest+".tar")
    initial=immutable.read_bytes()
    def attempt(path):
        return call(escrow, "app-source-stage-inactive",
                    digest, "core_configurator", path)
    for label,path in (
        ("dotdot-equivalent", escrow/".."/"escrow"),
        ("nested-output", descendant),
        ("ancestor-output", root),
    ):
        result=attempt(path)
        assert result.returncode!=0 and not result.stdout, (label,result.returncode,result.stdout)
        assert "HA_REGISTRY_STAGING_FAIL_CLOSED" in result.stderr
        assert not (path/f"app-core_configurator-{digest}").exists()
        assert immutable.read_bytes()==initial
        print(f"P8EG_SINGLE_{label.upper().replace('-', '_')}_OVERLAP_REJECTED_PASS")
    aliased = root/"linked-parent"
    aliased.symlink_to(escrow,target_is_directory=True)
    result=attempt(aliased/".")
    assert result.returncode!=0 and not result.stdout
    assert immutable.read_bytes()==initial
    print("P8EG_SINGLE_SYMLINKED_PARENT_ALIAS_REJECTED_PASS")
    ok=attempt(disjoint)
    assert ok.returncode==0,ok.stderr
    receipt=json.loads(ok.stdout)
    assert receipt["production_ready"] is False and receipt["app_runtime_started"] is False
    restored=disjoint/("app-core_configurator-"+digest)
    assert (restored/"data/p8eg-private-marker.txt").read_bytes()==marker
    assert immutable.read_bytes()==initial
    print("P8EG_SINGLE_DISJOINT_REAL_NATIVE_APP_STAGE_ORIGINAL_SHA_UNCHANGED_PASS")
