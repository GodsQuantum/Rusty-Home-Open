#!/usr/bin/env python3
"""P1f lab ONLY: separate Rust processes, SIGSTOP partition, explicit epoch handoff.
Synthetic redb sink, not the Moli driver or any hardware device.
"""
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile

ROOT=Path(__file__).resolve().parent.parent
if os.geteuid()==0 or not (
    os.uname().nodename=="dev-build" or os.environ.get("GITHUB_ACTIONS")=="true"
):
    raise SystemExit("unprivileged isolated Linux lab or CI only")
subprocess.run(["cargo","build","--quiet","--locked","-p","hars-native-kernel"],
               cwd=ROOT,timeout=300,check=True)
binary=ROOT/"target/debug/hars-native-kernel"
(ROOT/"runtime").mkdir(mode=0o700,exist_ok=True)
def execute(root, action, *args, allowed=True):
    result=subprocess.run([str(binary),"--lab-fence",str(root),action,*map(str,args)],
                          cwd=ROOT,timeout=8,text=True,capture_output=True)
    assert (result.returncode==0)==allowed, (action,result.stderr[:400])
    return json.loads(result.stdout) if allowed else None

with tempfile.TemporaryDirectory(dir=ROOT/"runtime",prefix="hars-fence-") as tmp:
    folder=Path(tmp)
    os.chmod(folder,0o700)
    first=execute(folder,"first","owner_a")
    assert first["value"]==1
    assert execute(folder,"apply","owner_a",1,"cmd_one","true")["committed"]
    assert not execute(folder,"apply","owner_a",1,"cmd_one","true")["committed"]
    execute(folder,"apply","owner_a",1,"cmd_one","false",allowed=False)
    execute(folder,"first","owner_b",allowed=False)
    execute(folder,"transfer","owner_a",99,"owner_b",allowed=False)
    print("PASS: first authority, idempotent effect and unsafe transfer denied")

    # Process is stopped after opening the DB and before applying the stale
    # operation, while another process legitimately increments the epoch.
    child=subprocess.Popen(
        [str(binary),"--lab-fence",str(folder),"delayed-apply",
         "owner_a","1","cmd_partition","false"],
        cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,
    )
    try:
        line=child.stdout.readline().strip()
        assert line=="LAB_FENCE_DEFERRED_READY",(line,child.poll())
        os.kill(child.pid,signal.SIGSTOP)
        new=execute(folder,"transfer","owner_a",1,"owner_b")
        assert new["value"]==2 and new["owner"]=="owner_b"
        os.kill(child.pid,signal.SIGCONT)
        out,err=child.communicate(timeout=8)
        assert child.returncode!=0,("stale worker dispatched after takeover",out,err)
        assert "stale controller fenced" in err
    finally:
        if child.poll() is None:
            os.kill(child.pid,signal.SIGCONT)
            child.kill()
            child.wait(timeout=5)
        child.stdout.close()
        child.stderr.close()
    state=execute(folder,"state")
    assert state["owner"]["value"]==2
    assert state["actuator"]["command_id"]=="cmd_one"
    assert state["actuator"]["requested_on"] is True
    assert execute(folder,"apply","owner_b",2,"cmd_new","false")["committed"]
    execute(folder,"apply","owner_a",1,"cmd_stale","true",allowed=False)
    print("PASS: real SIGSTOP/CONT split-brain fixture fences stale process; no old effect committed")

    reopened=execute(folder,"state")
    assert reopened["actuator"]["requested_on"] is False
    assert reopened["actuator"]["epoch"]==2
    assert reopened["owner"]["owner"]=="owner_b"
    assert reopened["owner"]["value"]==2
    assert (folder/"fence.redb").stat().st_mode & 0o077==0
    execute(folder,"transfer","owner_a",1,"owner_c",allowed=False)
    print("PASS: fresh process reopens durable redb; epoch monotonic and file private")
print("PASS: P1f all processes closed, disposable DB removed, no devices or network")
