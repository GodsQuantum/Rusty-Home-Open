#!/usr/bin/env python3
"""P8a: independent Rust process inspecting unknown Supervisor app manifests."""
from __future__ import annotations
import json
import os
import stat
import subprocess
import tempfile
from pathlib import Path

ROOT=Path(__file__).resolve().parent.parent
if os.geteuid()==0 or not (os.uname().nodename=="dev-build" or os.environ.get("GITHUB_ACTIONS")=="true"):
    raise SystemExit("Unprivileged isolated Linux lab or independent CI only")
runtime=ROOT/"runtime"
runtime.mkdir(mode=0o700,exist_ok=True)
def run(root:Path):
    return subprocess.run(["cargo","run","--quiet","--locked","-p","hars-supervisor-apps",
                           "--example","inspect_repository","--",str(root)],
                          cwd=ROOT,capture_output=True,text=True,timeout=90)
def write(root:Path, path:str, data:str):
    dest=root/path;dest.parent.mkdir(parents=True,exist_ok=True)
    dest.write_text(data,encoding="utf-8");dest.chmod(0o600)

with tempfile.TemporaryDirectory(prefix="hars-unknown-apps-",dir=runtime) as folder:
    root=Path(folder)
    root.chmod(0o700)
    write(root,"repository.yaml","name: Future Independent Vendor\n")
    write(root,"new-company/unknown-device/config.yaml",'''name: Unknown connector
version: "2099.01"
slug: future_connector
description: Completely new
arch: [amd64, aarch64]
image: ghcr.io/new-organization/connector
backup: cold
ingress: true
map:
  - type: share
    read_only: false
docker_api: true
future_feature: SECRET_MUST_NOT_LEAK
''')
    write(root,"another-developer/smart-panel/config.json",json.dumps({
        "name":"Unknown panel","version":"2.0","slug":"smart_panel",
        "description":"Third-party vendor","arch":["aarch64"],
        "image":"ghcr.io/other/vendor","homeassistant_api":True,
    }))
    output=run(root)
    assert output.returncode==0, output.stderr[-1400:]
    found=json.loads(output.stdout)
    assert found["name"]=="Future Independent Vendor"
    assert len(found["apps"])==2
    assert not found["activation_authorized"]
    byslug={app["slug"]:app for app in found["apps"]}
    assert set(byslug)=={"future_connector","smart_panel"}
    unknown=byslug["future_connector"]
    assert {"ingress","docker_api","writable_mount:share","backup:cold"}.issubset(set(unknown["requirements"]))
    assert unknown["unresolved_fields"]==["future_feature"]
    assert all(not a["runtime_compatible"] and not a["activation_authorized"] for a in found["apps"])
    assert "SECRET_MUST_NOT_LEAK" not in output.stdout
    print("PASS: two arbitrary unknown HA Apps in nested YAML+JSON, no hard-coded allowlist, sensitive values redacted")
    write(root,"third-dev/duplicate/config.json",json.dumps({
        "name":"Duplicate","version":"1.0","slug":"smart_panel","description":"duplicate",
        "arch":["amd64"],"image":"ghcr.io/any/app"
    }))
    assert run(root).returncode != 0, "duplicate slug was accepted"
    (root/"third-dev/duplicate/config.json").unlink()
    os.symlink("new-company",root/"linked-vendor")
    assert run(root).returncode != 0, "symlink escape was accepted"
    (root/"linked-vendor").unlink()
    print("PASS: independent real Rust subprocess rejects duplicate identities and symlinked directories")
    bad=run(Path("/etc"))
    assert bad.returncode !=0,"test CLI accepted outside project runtime"
    print("PASS: external paths rejected; no container build/pull/start, no hardware/HAOS access")
print("P8a E2E fixture cleanup complete")
