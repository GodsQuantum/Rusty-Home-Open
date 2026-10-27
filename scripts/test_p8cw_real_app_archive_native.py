#!/usr/bin/env python3
"""P8cw: official disposable HAOS File Editor PARTIAL archive to private
Rust sha256 escrow, independent process read-back; NO App activation/restore.
"""
import argparse,hashlib,json,os,subprocess,tempfile
from pathlib import Path
from inspect_ha_backup import inspect_backup
p=argparse.ArgumentParser()
p.add_argument("--archive",type=Path,required=True)
p.add_argument("--manifest",type=Path,required=True)
p.add_argument("--native",type=Path,required=True)
args=p.parse_args()
archive=args.archive.resolve(strict=True)
native=args.native.resolve(strict=True)
manifest=json.loads(args.manifest.read_text())
assert archive.stat().st_size==manifest["size_bytes"]
with archive.open("rb") as src:
    digest=hashlib.file_digest(src,"sha256").hexdigest()
assert digest==manifest["sha256"]
pre=inspect_backup(archive,{"core_configurator"})
assert pre["app_only_partial_archive_identified"] is True
assert pre["blocking_issues"]==[]
assert pre["can_restore"] is False
root=Path(__file__).resolve().parent.parent/"runtime"
root.mkdir(mode=0o700,exist_ok=True)
assert not root.is_symlink() and root.stat().st_mode & 0o077==0
with tempfile.TemporaryDirectory(prefix="p8cw-real-app-only-",dir=root) as tmp:
    escrow=Path(tmp)
    assert escrow.stat().st_mode & 0o077==0
    def run(*commands,ok=True):
        proc=subprocess.run([str(native),"--lab-readonly",str(escrow),*map(str,commands)],
                            capture_output=True,text=True,timeout=65)
        if not ok:
            assert proc.returncode!=0
            assert "HA_REGISTRY_STAGING_FAIL_CLOSED" in proc.stderr
            assert not proc.stdout
            return None
        assert proc.returncode==0,proc.stderr[-400:]
        assert "SUPERVISOR_TOKEN" not in proc.stdout
        return json.loads(proc.stdout)
    first=run("app-source-ingest",archive)
    assert first["action"]=="private_app_backup_escrow_ingest"
    assert first["sha256"]==digest
    assert first["bytes"]==archive.stat().st_size
    assert first["app_count"]==1 and first["files"]>=1
    assert first["declared_payload_bytes"]>=65
    assert first["read_only"] and first["apps_restored"] is False
    assert first["app_runtime_migrated"] is False
    assert first["actions_executable"] is False
    assert first["production_ready"] is False
    assert first==run("app-source-ingest",archive)
    print("P8CW_GENUINE_SUPERVISOR_APP_ONLY_PRIVATE_RUST_SHA_INGEST_PASS")
    second=run("app-source-verify",digest)
    assert second["archive_sha256"]==digest
    assert second["app_count"]==first["app_count"]
    assert second["files"]==first["files"]
    assert second["declared_payload_bytes"]==first["declared_payload_bytes"]
    assert second["production_ready"] is False
    print("P8CW_APP_ONLY_GENUINE_HAOS_ARCHIVE_RUST_REOPEN_AND_INERT_READBACK_PASS")
    run("app-source-verify","f"*64,ok=False)
    run("app-source-verify","../"+digest,ok=False)
    saved=escrow/(digest+".tar")
    assert saved.stat().st_mode & 0o077==0
    assert saved.stat().st_size==archive.stat().st_size
    with saved.open("r+b") as f:
        f.seek(saved.stat().st_size-1024)
        previous=f.read(1)
        f.seek(saved.stat().st_size-1024)
        f.write(bytes((previous[0]^1,)))
        f.flush();os.fsync(f.fileno())
    run("app-source-verify",digest,ok=False)
    with archive.open("rb") as f:
        assert hashlib.file_digest(f,"sha256").hexdigest()==digest
    print("P8CW_APP_ONLY_GENUINE_HAOS_ESCROW_TAMPER_FAIL_CLOSED_ORIGINAL_UNCHANGED_PASS")
