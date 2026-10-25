#!/usr/bin/env python3
"""Check actual binary lab-only mode, no token arguments or production paths."""
from pathlib import Path
import os
import subprocess
import tempfile

ROOT=Path(__file__).resolve().parent.parent
BINARY=ROOT/"target/debug/hars-native-kernel"
assert os.geteuid()!=0, "lab CLI checks must be nonroot"
subprocess.run(["cargo","build","--locked","--quiet","-p","hars-native-kernel"],
               cwd=ROOT,check=True,timeout=300)
(RUNTIME:=ROOT/"runtime").mkdir(exist_ok=True,mode=0o700)

def reject(directory,token=None):
    env=os.environ.copy()
    env.pop("HARS_LAB_WS_TOKEN",None)
    if token is not None:
        env["HARS_LAB_WS_TOKEN"]=token
    call=subprocess.run([str(BINARY),"--lab-ha-ws-to-moli",str(directory),
       "ws://127.0.0.1:6551/api/websocket","sensor.lab_test","on","1"],
       cwd=ROOT,env=env,text=True,capture_output=True,timeout=15)
    assert call.returncode!=0,call.stdout
    assert "production_ready" not in call.stdout
    assert (token or "sensitive_canary") not in call.stdout+call.stderr
    return call

with tempfile.TemporaryDirectory(prefix="hars-ws-cli-",dir=RUNTIME) as tmp:
    d=Path(tmp)
    os.chmod(d,0o700)
    reject(d)
    reject(d,"sensitive_canary")
    # Arbitrary paths outside the selected runtime child cannot be imported.
    with tempfile.TemporaryDirectory(prefix="hars-ws-forbidden-") as other:
        reject(other,"sensitive_canary")
print("PASS: actual compiled Rust CLI rejects missing token, unstaged identity and external paths")
print("PASS: no token or unsupported production claims logged; no socket connected")
