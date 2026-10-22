#!/usr/bin/env python3
"""Real Linux-process exclusive writer admission, synthetic radio fixture only."""
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import tempfile

ROOT=Path(__file__).resolve().parent.parent
assert os.geteuid()!=0, "Radio writer lab cannot run as root"
assert os.uname().nodename=="dev-build" or os.environ.get("GITHUB_ACTIONS")=="true"
binary=Path(os.environ.get("CARGO_TARGET_DIR",str(ROOT/"target")))/"debug/hars-native-kernel"
assert binary.is_file(), "Build the native Rust kernel before this test"
runtime=ROOT/"runtime"
runtime.mkdir(mode=0o700,exist_ok=True)

def probe(path:Path, *, good:bool):
    result=subprocess.run([str(binary),"--lab-radio-lease",str(path),"probe"],
      cwd=ROOT,capture_output=True,text=True,timeout=4)
    if good:
        assert result.returncode==0,(result.returncode,result.stderr[-300:])
        report=json.loads(result.stdout)
        assert report=={"exclusive_writer":True,"lab_only":True,
          "real_radio_access":False,"production_ready":False},report
    else:
        assert result.returncode!=0,(path,result.stdout)
        assert "exclusive_writer" not in result.stdout
    return result

with tempfile.TemporaryDirectory(prefix="hars-radio-lease-",dir=runtime) as temp:
    folder=Path(temp)
    folder.chmod(0o700)
    process=None
    try:
        probe(folder,good=True)
        process=subprocess.Popen([str(binary),"--lab-radio-lease",str(folder),"hold"],
            cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,bufsize=1)
        available,_,_=select.select([process.stdout],[],[],3)
        assert available, "Rust lease holder not ready"
        report=json.loads(process.stdout.readline())
        assert report["exclusive_writer"] and report["lab_only"]
        assert not report["real_radio_access"] and not report["production_ready"]
        assert process.poll() is None
        probe(folder,good=False)
        # Each private test fixture has a separate OS mutex. No hardware names.
        other=folder.parent/(folder.name+"-sibling")
        other.mkdir(mode=0o700)
        try:probe(other,good=True)
        finally:other.rmdir() if not any(other.iterdir()) else __import__("shutil").rmtree(other)
        # SIGKILL unlocks automatically; no stale PID file override.
        process.send_signal(signal.SIGKILL)
        process.wait(timeout=4)
        process.stdout.close()
        process.stderr.close()
        process=None
        probe(folder,good=True)
        # Invalid file permissions deny an otherwise available lock.
        lock=folder/"synthetic-radio-owner.lock"
        lock.chmod(0o644)
        probe(folder,good=False)
        lock.chmod(0o600)
        probe(folder,good=True)
        # Links cannot redirect this into a privileged target.
        lock.unlink()
        trap=folder/"dummy-secret-sentinel"
        trap.write_text("UNTOUCHED")
        trap.chmod(0o600)
        lock.symlink_to(trap)
        probe(folder,good=False)
        assert trap.read_text()=="UNTOUCHED"
        lock.unlink()
        # Writable-to-group guard folder denied.
        folder.chmod(0o770)
        probe(folder,good=False)
        folder.chmod(0o700)
        probe(folder,good=True)
        alias=folder.parent/"hars-radio-p20-symlink"
        alias.symlink_to(folder, target_is_directory=True)
        try: probe(alias,good=False)
        finally:alias.unlink()
        print("P20_PASS: true Unix process exclusion, fail-closed input, SIGKILL release, zero radios")
    finally:
        if process is not None:
            process.kill()
            process.wait(timeout=4)
            process.stdout.close()
            process.stderr.close()
