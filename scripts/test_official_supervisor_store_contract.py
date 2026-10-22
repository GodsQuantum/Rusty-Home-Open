#!/usr/bin/env python3
"""Independent Rust process parses real-shaped official Supervisor Store V1/V2.

Fixtures derive from home-assistant/supervisor supervisor/api/store.py at pinned
ca2273e2a7ae061a7377658fb704a0b919330fb8, but contain no real tokens.
"""
from __future__ import annotations
import json
import os
from pathlib import Path
import subprocess
ROOT=Path(__file__).resolve().parent.parent
assert os.geteuid()!=0 and (os.uname().nodename=="dev-build" or os.environ.get("GITHUB_ACTIONS")=="true")
subprocess.run(["cargo","build","--locked","--quiet","-p","hars-supervisor-apps",
                "--example","parse_official_store"],cwd=ROOT,check=True,timeout=180)
binary=ROOT/"target/debug/examples/parse_official_store"
unknown_app={"slug":"b123abcd_unknown_future","repository":"b123abcd","name":"Future vendor",
             "available":True,"installed":False,"update_available":False,
             "url":"https://example.invalid/?api_token=do-not-expose"}
repo={"slug":"b123abcd","name":"Future unofficial vendor",
      "source":"https://example.invalid/token=never-log"}
for variant in ("addons","apps"):
    fixture={"result":"ok","data":{variant:[unknown_app],"repositories":[repo]}}
    completed=subprocess.run([str(binary)],input=json.dumps(fixture),text=True,
                             capture_output=True,timeout=5,cwd=ROOT)
    assert completed.returncode==0,completed.stderr
    data=json.loads(completed.stdout)
    assert data["api_version"]==("V1" if variant=="addons" else "V2")
    assert data["apps"][0]["slug"]=="b123abcd_unknown_future"
    assert data["repositories"][0]["slug"]=="b123abcd"
    assert data["official_supervisor_owner"]
    assert not data["activation_authorized"]
    assert "token=" not in completed.stdout and "https://" not in completed.stdout
    print(f"PASS: genuine-shaped official Supervisor {variant} catalogue, unknown App, no secrets and no install rights")
for invalid in [
    {"result":"error","data":{"apps":[],"repositories":[]}},
    {"apps":[],"addons":[],"repositories":[]},
    {"apps":[unknown_app,unknown_app],"repositories":[repo]},
]:
    failed=subprocess.run([str(binary)],input=json.dumps(invalid),text=True,
                          capture_output=True,timeout=5,cwd=ROOT)
    assert failed.returncode!=0
print("PASS: negative responses, ambiguous versions and duplicates refused")
print("P8b OFFICIAL SUPERVISOR STORE CONTRACT — READ-ONLY LAB ONLY")
