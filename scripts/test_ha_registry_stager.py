#!/usr/bin/env python3
"""Real independent Rust process proof for safe read-only HA registry staging.

Creates fictional Home Assistant Core .storage-style registry data ONLY.
Never touches HAOS, a household backup, credentials, actuators or Home Assistant.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT=Path(__file__).resolve().parent.parent
BIN=ROOT/"target/debug/ha_registry_stager"
subprocess.run(["cargo","build","--locked","-p","hars-interop-moli",
                "--bin","ha_registry_stager","--quiet"],cwd=ROOT,
               timeout=250,check=True)
with tempfile.TemporaryDirectory(prefix="hars-registry-stager-") as work:
    base=Path(work)
    private=base/"shadow-private"
    private.mkdir(mode=0o700)
    def make(name,items):
        path=base/name
        data={"version":1,"minor_version":15,"key":"core.entity_registry",
              "data":{"entities":items}}
        path.write_text(json.dumps(data))
        return path
    def item(i):
        # Any entity domain/provider is accepted; this is no vendor allowlist.
        domain=("sensor","switch","binary_sensor","light","cover","climate")[i%6]
        return {"entity_id":f"{domain}.third_party_{i}",
                "platform":f"vendor_{i%19}",
                "unique_id":f"vendor/serial-{i}",
                "config_entry_id":f"config-entry-{i%17}",
                "device_id":f"device-{i%97}","disabled_by":None}
    old=make("registry-old.json",[item(i) for i in range(1232)])
    new=make("registry-new.json",[item(i) for i in range(1233)])
    def call(*params,deny=False):
        out=subprocess.run([str(BIN),"--lab-readonly",str(private),*map(str,params)],
            cwd=ROOT,text=True,capture_output=True,timeout=30)
        if deny:
            assert out.returncode!=0,out.stdout
            assert "HA_REGISTRY_STAGING_FAIL_CLOSED" in out.stderr
            assert "vendor/serial" not in out.stderr
            return None
        assert out.returncode==0,out.stderr[-300:]
        return json.loads(out.stdout)
    assert call("status")["active"] is None
    first=call("stage",old)
    assert first["entities"]==1232 and first["stable_identities"]==1232
    assert first["read_only"] and not first["live_states_migrated"]
    assert first==call("stage",old)
    second=call("stage",new)
    assert second["entities"]==1233 and second["sha256"]!=first["sha256"]
    assert call("status")["active"] is None, "prepare must not activate"
    call("activate",first["sha256"],"none")
    assert call("status")["active"]==first["sha256"],"separate Rust process lost state"
    call("activate",second["sha256"],"none",deny=True)
    call("activate",second["sha256"],first["sha256"])
    assert call("status")["entities"]==1233
    call("rollback",first["sha256"],deny=True)
    previous=call("rollback",second["sha256"])
    assert previous["restored"]==first["sha256"]
    assert call("status")["active"]==first["sha256"]
    assert call("status")["no_driver_commands"] and not call("status")["production_ready"]
    assert (private/"registry-shadow.redb").stat().st_mode&0o077==0
    print("PASS: 1232 arbitrary-vendor registry entries persisted across independent Rust processes,")
    print("      CAS activation, upgrade, wrong-target rejection, full rollback and owner-private storage")
    bad=base/"duplicate.json"
    bad.write_text('{"key":"core.entity_registry","version":1,"data":{"entities":[{"entity_id":"sensor.a","entity_id":"sensor.b","platform":"mqtt","unique_id":"x"}]}}')
    call("stage",bad,deny=True)
    link=base/"symlink.json"
    link.symlink_to(old)
    call("stage",link,deny=True)
    hardlink=base/"hardlink.json"
    os.link(old,hardlink)
    call("stage",hardlink,deny=True)
    fifo=base/"fifo.json"
    os.mkfifo(fifo)
    call("stage",fifo,deny=True)
    call("activate","x"*64,first["sha256"],deny=True)
    assert call("status")["active"]==first["sha256"]
    print("PASS: duplicate metadata, symlinks, hardlinks, FIFO and unstaged digests denied")
print("PASS: isolated temporary fixture destroyed, no household or external App touched")
