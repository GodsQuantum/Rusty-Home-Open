#!/usr/bin/env python3
"""P8dd: real SIGKILL of native Rust offline stage, never App execution."""
import argparse, hashlib, io, json, os, signal, stat, subprocess, tarfile, tempfile, time
from pathlib import Path
parser = argparse.ArgumentParser()
parser.add_argument("--binary", required=True, type=Path)
binary = parser.parse_args().binary.resolve(strict=True)
first, second = b"P8DD_FIRST_FILE\n", b"P8DD_SECOND_FILE\n"

def fixture(source):
    part = io.BytesIO()
    with tarfile.open(fileobj=part, mode="w:gz") as tar:
        for name, data in (("data/a-first.bin", first), ("data/z-second.bin", second)):
            item = tarfile.TarInfo("./"+name)
            item.size = len(data)
            tar.addfile(item, io.BytesIO(data))
    with tarfile.open(source, "w") as tar:
        for name, data in (
            ("backup.json", json.dumps({"type":"partial", "protected":False, "addons":[{"slug":"core_configurator"}]}).encode()),
            ("core_configurator.tar.gz", part.getvalue()),
        ):
            item = tarfile.TarInfo("./"+name)
            item.size = len(data)
            tar.addfile(item, io.BytesIO(data))
    source.chmod(0o600)

def cmd(escrow, *args, check=True):
    p = subprocess.run([str(binary),"--lab-readonly",str(escrow),*map(str,args)], capture_output=True, text=True, timeout=45)
    if check:
        assert p.returncode==0,(args,p.stderr)
        return json.loads(p.stdout)
    assert p.returncode!=0 and "HA_REGISTRY_STAGING_FAIL_CLOSED" in p.stderr,(args,p.stdout,p.stderr)

def slow(escrow, digest, root):
    env = dict(os.environ,HARS_P8DD_LAB_PAUSE_AFTER_FIRST_FILE_MS="30000")
    p = subprocess.Popen([str(binary),"--lab-readonly",str(escrow),"app-source-stage-inactive",digest,"core_configurator",str(root)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env)
    end=time.monotonic()+15
    while time.monotonic()<end:
        dirs=[x for x in root.glob(".inactive-app-*") if (x/".hars-stage-lease-v1").is_file() and (x/"data/a-first.bin").is_file()]
        if dirs:
            assert p.poll() is None
            assert (dirs[0]/"data/a-first.bin").read_bytes()==first
            assert not (dirs[0]/"data/z-second.bin").exists()
            return p,dirs[0]
        if p.poll() is not None: raise AssertionError(("early native exit",p.communicate()))
        time.sleep(.05)
    p.kill();p.wait(timeout=10)
    raise AssertionError("first staged file never appeared")

with tempfile.TemporaryDirectory(prefix="p8dd-") as work:
    base=Path(work);escrow=base/"escrow";escrow.mkdir(mode=0o700)
    source=base/"fixture.tar";fixture(source)
    digest=hashlib.sha256(source.read_bytes()).hexdigest()
    cmd(escrow,"app-source-ingest",source)
    assert cmd(escrow,"app-source-verify",digest)["archive_sha256"]==digest
    target=base/"inactive";target.mkdir(mode=0o700)
    unrelated=target/".inactive-app-not-ours";unrelated.mkdir(mode=0o700)
    (unrelated/"keep").write_bytes(b"untouched")
    forged=target/".inactive-app-forged";forged.mkdir(mode=0o700)
    (forged/".hars-stage-lease-v1").write_bytes(b"HARS_INACTIVE_APP_STAGE_V1\n")
    (forged/".hars-stage-lease-v1").chmod(0o644)  # reject nonprivate forged marker
    symlinked=target/".inactive-app-symlink";symlinked.mkdir(mode=0o700)
    (symlinked/".hars-stage-lease-v1").symlink_to(forged/".hars-stage-lease-v1")
    p,orphan=slow(escrow,digest,target)
    p.send_signal(signal.SIGKILL);assert p.wait(timeout=10)==-signal.SIGKILL
    assert orphan.is_dir() and not (target/("app-core_configurator-"+digest)).exists()
    assert (orphan/"data/a-first.bin").read_bytes()==first
    print("P8DD_REAL_NATIVE_STAGE_SIGKILL_AFTER_FIRST_DURABLE_FILE_NO_PUBLISH_PASS",flush=True)
    receipt=cmd(escrow,"app-source-stage-inactive",digest,"core_configurator",target)
    final=target/("app-core_configurator-"+digest)
    assert receipt["files"]==2 and receipt["production_ready"] is False
    assert not orphan.exists() and (unrelated/"keep").read_bytes()==b"untouched"
    assert forged.is_dir() and symlinked.is_dir()  # never delete unknown user dirs
    assert (final/"data/a-first.bin").read_bytes()==first
    assert (final/"data/z-second.bin").read_bytes()==second
    assert not (final/".hars-stage-lease-v1").exists()
    for f in (final,final/"data",final/"data/a-first.bin",final/"data/z-second.bin"):
        assert stat.S_IMODE(f.stat().st_mode)&0o077==0
    print("P8DD_NATIVE_RETRY_REAPS_ONLY_OWN_SIGKILL_ORPHAN_AND_RESTORES_EXACT_BYTES_PASS",flush=True)
    concurrent=base/"concurrent";concurrent.mkdir(mode=0o700)
    escrow2=base/"escrow-second-independent-writer";escrow2.mkdir(mode=0o700)
    cmd(escrow2,"app-source-ingest",source)
    p,active=slow(escrow,digest,concurrent)
    receipt=cmd(escrow2,"app-source-stage-inactive",digest,"core_configurator",concurrent)
    assert receipt["files"]==2 and active.exists() and p.poll() is None
    print("P8DD_NATIVE_CONCURRENT_WRITER_LOCK_PREVENTS_ACTIVE_STAGE_REAP_PASS",flush=True)
    p.kill();assert p.wait(timeout=10)==-signal.SIGKILL
    cmd(escrow2,"app-source-stage-inactive",digest,"core_configurator",concurrent,check=False)
    assert not active.exists()
    assert (concurrent/("app-core_configurator-"+digest)/"data/z-second.bin").read_bytes()==second
    assert hashlib.sha256(source.read_bytes()).hexdigest()==digest
    print("P8DD_NATIVE_EXISTING_PUBLISHED_DATA_NO_OVERWRITE_AFTER_ORPHAN_REAP_PASS",flush=True)
    print("P8DD_NOT_PRODUCTION_READY_NO_APP_ACTIVATION_NO_VM100_PASS",flush=True)
