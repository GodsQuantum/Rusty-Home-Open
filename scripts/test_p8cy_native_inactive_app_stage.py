#!/usr/bin/env python3
"""Native Rust offline App-data restore into empty private target, fault-injected.

Synthetic fixtures for fast local CI; do not touch OCI/HAOS/user secrets.
"""
import argparse,hashlib,io,json,os,stat,subprocess,tarfile,tempfile
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument("--binary",type=Path,required=True)
opt=p.parse_args()
binary=opt.binary.resolve(strict=True)
marker=b"P8CY_SYNTHETIC_NATIVE_APP_DATA_AT_REST\n"
def outer(path,*,duplicate=False,evil=None):
    buf=io.BytesIO()
    with tarfile.open(fileobj=buf,mode="w:gz") as inner:
        def put(path,body,kind=tarfile.REGTYPE):
            info=tarfile.TarInfo("./"+path)
            info.type=kind
            info.size=len(body) if kind==tarfile.REGTYPE else 0
            if kind==tarfile.SYMTYPE:info.linkname="../../escape"
            inner.addfile(info,io.BytesIO(body) if kind==tarfile.REGTYPE else None)
        put("data/config/test-state.bin",marker)
        put("data/symlink" if evil=="link" else "data/another.bin",
            b"fake" if evil!="link" else b"",tarfile.SYMTYPE if evil=="link" else tarfile.REGTYPE)
        if duplicate:put("data/config/test-state.bin",marker)
    with tarfile.open(path,"w") as tf:
        def item(name,data):
            entry=tarfile.TarInfo("./"+name)
            entry.size=len(data)
            tf.addfile(entry,io.BytesIO(data))
        item("backup.json",json.dumps({"type":"partial","protected":False,
                                       "addons":[{"slug":"core_configurator"}]}).encode())
        item("core_configurator.tar.gz",buf.getvalue())
    path.chmod(0o600)

with tempfile.TemporaryDirectory(prefix="p8cy-") as tmp:
    root=Path(tmp)
    escrow=root/"escrow";escrow.mkdir(mode=0o700)
    inactive=root/"inactive";inactive.mkdir(mode=0o700)
    source=root/"source.tar"
    outer(source)
    digest=hashlib.sha256(source.read_bytes()).hexdigest()
    def run(command,*args,ok=True,env=None):
        result=subprocess.run([str(binary),"--lab-readonly",str(escrow),command,
                               *map(str,args)],capture_output=True,text=True,timeout=40,env=env)
        if not ok:
            assert result.returncode!=0,(command,result.stdout)
            assert "HA_REGISTRY_STAGING_FAIL_CLOSED" in result.stderr
            assert not result.stdout
            return
        assert result.returncode==0,(command,result.stderr)
        data=json.loads(result.stdout)
        assert data["production_ready"] is False and "P8CY_SYNTHETIC_NATIVE" not in result.stdout
        return data
    run("app-source-ingest",source)
    final=inactive/("app-core_configurator-"+digest)
    assert not final.exists()
    failure_env=dict(os.environ,HARS_P8CY_FAIL_BEFORE_COMMIT="1")
    run("app-source-stage-inactive",digest,"core_configurator",inactive,ok=False,env=failure_env)
    assert not final.exists()
    assert list(inactive.iterdir())==[]
    assert run("app-source-verify",digest)["archive_sha256"]==digest
    print("P8CY_NATIVE_APP_STAGE_INJECTED_FAILURE_ROLLS_BACK_ZERO_FILES_PASS")
    report=run("app-source-stage-inactive",digest,"core_configurator",inactive)
    assert report["sha256"]==digest and report["files"]==2 and report["app_count"]==1
    assert report["app_image_installed"] is False and report["app_runtime_started"] is False
    assert report["automation_executed"] is False and report["staged_private_regular_files"]
    path=final/"data/config/test-state.bin"
    assert path.read_bytes()==marker
    assert report["bytes"]==len(marker)+len(b"fake")
    for item in (final,final/"data",final/"data/config",path):
        assert item.stat().st_mode & 0o077==0,(item,oct(item.stat().st_mode))
        assert not item.is_symlink()
    assert run("app-source-verify",digest)["archive_sha256"]==digest
    run("app-source-stage-inactive",digest,"core_configurator",inactive,ok=False)
    assert path.read_bytes()==marker
    assert len(list(inactive.iterdir()))==1
    print("P8CY_NATIVE_APP_INACTIVE_PRIVATE_ATOMIC_PUBLISH_NO_OVERWRITE_PASS")
    run("app-source-stage-inactive",digest,"not_declared",inactive,ok=False)
    run("app-source-stage-inactive",digest,"core_configurator",escrow,ok=False)
    bad=root/"world";bad.mkdir(mode=0o755)
    run("app-source-stage-inactive",digest,"core_configurator",bad,ok=False)
    symlink=root/"target-link";symlink.symlink_to(inactive,target_is_directory=True)
    run("app-source-stage-inactive",digest,"core_configurator",symlink,ok=False)
    blob=escrow/(digest+".tar")
    saved=blob.read_bytes()
    blob.write_bytes(saved[:1024]+b"tampered"+saved[1032:])
    fresh=root/"private-tamper-target"
    fresh.mkdir(mode=0o700)
    run("app-source-stage-inactive",digest,"core_configurator",fresh,ok=False)
    assert list(fresh.iterdir())==[],"tampered escrow leaked untrusted native files"
    run("app-source-verify",digest,ok=False)
    assert path.read_bytes()==marker
    print("P8CY_NATIVE_APP_SHA_REFUSAL_BAD_UID_TARGET_SYMLINK_ORIGINAL_STAGE_UNCHANGED_PASS")
