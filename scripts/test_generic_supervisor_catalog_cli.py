#!/usr/bin/env python3
"""End-to-end read-only App catalog CLI: 1,317 unknown Apps, 13 repos."""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXE = ROOT / "target/debug/examples/parse_official_store"
subprocess.run(
    ["cargo","build","--quiet","--locked","-p","hars-supervisor-apps",
     "--example","parse_official_store"],
    cwd=ROOT,check=True,
)
apps = [
    {"slug":f"future_{i:05}","name":f"Future Product {i}",
     "repository":f"unknown_{i%13}","available":True,
     "installed":i%2==0,"update_available":i%3==0}
    for i in range(1317)
]
repos = [{"slug":f"unknown_{i}","name":f"Unknown Provider {i}"} for i in range(13)]
payload=json.dumps({"result":"ok","data":{"apps":apps,"repositories":repos}}).encode()

def call(*args):
    return subprocess.run([str(EXE),*args],input=payload,capture_output=True,check=False)

all_seen=[]
offset=0
while True:
    result=call("--limit","103","--offset",str(offset))
    assert result.returncode==0,result.stderr.decode()
    page=json.loads(result.stdout)
    assert page["total"]==1317 and page["activation_authorized"] is False
    all_seen.extend(a["slug"] for a in page["apps"])
    offset=page["next_offset"]
    if offset is None:break
assert all_seen==sorted(a["slug"] for a in apps)
assert len(set(all_seen))==1317

filtered=call("--repository","UNKNOWN_3","--installed","true","--updates-only","--limit","200")
assert filtered.returncode==0, filtered.stderr.decode()
page=json.loads(filtered.stdout)
assert page["total"]>0
assert all(a["repository"]=="unknown_3" and a["installed"] and a["update_available"] for a in page["apps"])
for invalid in [
    ("--limit","0"),("--limit","201"),("--installed","invalid"),
    ("--bogus",),("--limit","notanumber"),("--limit","7","--limit","8"),
]:
    result=call(*invalid)
    assert result.returncode!=0,(invalid,result.stdout)
    assert not result.stdout
assert call("--text","FUTURE PRODUCT 1316","--limit","5").returncode==0
print("GENERIC_SUPERVISOR_CATALOG_CLI_PASS: 1317 arbitrary Apps / 13 providers, exact paging, search and filtering")


# Product regression: never hide unknown Apps or classify them by a personal
# collection of known brand names.
html=(ROOT/"web/index.html").read_text()
server=(ROOT/"crates/control-plane/src/main.rs").read_text()
assert '<option value="all" selected>' in html
assert 'a.backend===select("#filter").value' in html
assert 'Apps reconnues' not in html
assert 'fn app_kind(' not in server
assert '"app_type": "generic_container"' in server
assert '"app_type": "supervisor_app"' in server
print("UNIVERSAL_CONTROL_PLANE_UI_PASS: all unknown providers visible by default")
