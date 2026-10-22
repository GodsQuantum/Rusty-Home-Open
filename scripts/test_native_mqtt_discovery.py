#!/usr/bin/env python3
"""Real-process offline Rust MQTT discovery preview contract; NO MQTT broker."""
from __future__ import annotations
import json
import os
from pathlib import Path
import subprocess
import tempfile
ROOT=Path(__file__).resolve().parent.parent
assert os.geteuid()!=0
assert os.uname().nodename=="dev-build" or os.environ.get("GITHUB_ACTIONS")=="true"
subprocess.run(["cargo","build","--locked","-p","hars-native-kernel","--quiet"],cwd=ROOT,check=True,timeout=240)
exe=ROOT/"target/debug/hars-native-kernel"
(ROOT/"runtime").mkdir(mode=0o700,exist_ok=True)

with tempfile.TemporaryDirectory(dir=ROOT/"runtime",prefix="hars-mqtt-preview-") as folder:
    local=Path(folder)
    os.chmod(local,0o700)
    payload=local/"mqtt.json"
    def run(topic:str,content:bytes,retained:bool=True,accept:bool=True):
        payload.write_bytes(content)
        response=subprocess.run([str(exe),"--lab-mqtt-discovery",topic,str(payload),
                                 str(retained).lower()],cwd=ROOT,capture_output=True,text=True,timeout=8)
        assert (response.returncode==0)==accept,(
            "unexpected Rust parser result",response.returncode,response.stderr)
        if accept:
            parsed=json.loads(response.stdout)
            assert parsed["read_only"] and parsed["native_rust"] and parsed["no_broker"]
            assert not parsed["production_ready"]
            return parsed
        assert not response.stdout, "rejected payload emitted structured data"
        return None
    device={"dev":{"ids":"synthetic-kitchen","name":"Lab Kitchen"},
            "o":{"name":"isolated-test"},
            "cmps":{"temp":{"p":"sensor","unique_id":"synthetic_temp"},
                    "motion":{"p":"binary_sensor","unique_id":"synthetic_motion"}},
            "state_topic":"isolated/test/sensor"}
    v=run("homeassistant/device/kitchen/config",json.dumps(device).encode())
    assert v["kind"]=="preview" and v["sensors"]==2
    print("PASS: 2 synthetic sensor records via real Rust CLI, zero MQTT connections")
    single={"unique_id":"synthetic_one","state_topic":"isolated/single"}
    v=run("homeassistant/sensor/bedroom/temp/config",json.dumps(single).encode(),retained=False)
    assert v["kind"]=="preview" and v["sensors"]==1
    assert run("homeassistant/sensor/bedroom/temp/config",b"")["kind"]=="remove_intent"
    assert run("homeassistant/sensor/bedroom/temp/config",b'{"migrate_discovery":true}',
               retained=False)["kind"]=="migration_intent"
    print("PASS: single component, retained deletion and migration intent")
    run("homeassistant/sensor/+/config",b"",accept=False)
    run("homeassistant/light/kitchen/config",json.dumps(single).encode(),accept=False)
    run("homeassistant/sensor/t/config",b"",retained=False,accept=False)
    device["cmps"]["motion"]["unique_id"]="synthetic_temp"
    run("homeassistant/device/kitchen/config",json.dumps(device).encode(),accept=False)
    print("PASS: wildcards, unsupported drivers, unretained delete, duplicate identities denied")
print("PASS: disposable fixture deleted, no daemon or external network")
