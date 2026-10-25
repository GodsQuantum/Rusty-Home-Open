#!/usr/bin/env python3
"""P8ab executable private escrow tests (fully synthetic, not real HAOS)."""
import hashlib,json,os
from pathlib import Path
import subprocess,tempfile

binary=Path(__import__("sys").argv[1]).resolve(strict=True)
with tempfile.TemporaryDirectory(prefix="hars-p8ab-escrow-") as root:
    base=Path(root)
    stage=base/"stage";stage.mkdir(mode=0o700)
    exported=base/"target";exported.mkdir(mode=0o700)
    src=base/"fake-sealed.tar"
    payload=(b"opaque_backups_are_not_automatically_restored" * 200)
    src.write_bytes(payload)
    src.chmod(0o600)
    def call(args,okay=True):
        res=subprocess.run([str(binary),"--lab-readonly",str(stage),*map(str,args)],
                           capture_output=True,text=True,timeout=30)
        if okay:
            assert res.returncode==0,(args,res.stderr)
            assert "secret" not in res.stdout
            return json.loads(res.stdout)
        assert res.returncode!=0,(args,res.stderr)
        assert "HA_REGISTRY_STAGING_FAIL_CLOSED" in res.stderr
        return None
    sha=hashlib.sha256(payload).hexdigest()
    a=call(["escrow-ingest",src])
    assert a=={"sha256":sha,"bytes":len(payload),"byte_exact":True,
        "opaque_no_decryption":True,"encrypted_supported_as_opaque":True,
        "restored_core_runtime":False,"migrated_automations":False,
        "migrated_apps":False,"production_ready":False},a
    assert call(["escrow-ingest",src])==a
    assert (stage/(sha+".tar")).read_bytes()==payload
    assert call(["escrow-export",sha,exported])==a
    assert (exported/"archive.tar").read_bytes()==payload
    call(["escrow-export",sha,exported],False)
    call(["escrow-export","e"*64,exported],False)
    call(["escrow-export","../"+sha,exported],False)
    symlink=base/"link.tar";symlink.symlink_to(src)
    call(["escrow-ingest",symlink],False)
    src.chmod(0o644)
    call(["escrow-ingest",src],False)
    src.chmod(0o600)
    src.write_bytes(payload+b"changed") 
    assert call(["escrow-ingest",src])["sha256"]!=sha
    stored=stage/(sha+".tar")
    stored.write_bytes(b"x"*len(payload))
    nextdir=base/"next";nextdir.mkdir(mode=0o700)
    call(["escrow-export",sha,nextdir],False)
    assert not (nextdir/"archive.tar").exists()
    assert (exported/"archive.tar").read_bytes()==payload
    (stage/(sha+".tar")).unlink()
    stored.symlink_to(src)
    call(["escrow-export",sha,nextdir],False)
    print("P8AB_OPAQUE_ARCHIVE_RUST_BYTE_EXACT_ESCROW_RESTORE_PASS")
    print("P8AB_TAMPER_SYMLINK_PERMISSIONS_NO_OVERWRITE_FAIL_CLOSED_PASS")
