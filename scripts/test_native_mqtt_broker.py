#!/usr/bin/env python3
"""Real Rust MQTT client + real Mosquitto 2.x over Docker --network none.

All hardware, user networks, volume mounts, credentials and Docker ports absent.
The only mounted payloads are synthetic fixture files and the test executable.
"""
from __future__ import annotations
import json
import os
import select
from pathlib import Path
import socket
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent
IMAGE = "hars-mqtt-lab:20261019"
if os.geteuid() == 0 or not (
    os.uname().nodename == "dev-build"
    or os.environ.get("GITHUB_ACTIONS") == "true"
):
    raise SystemExit("Only unprivileged isolated Linux lab or isolated CI")
subprocess.run(["cargo", "build", "-p", "hars-native-kernel", "--locked", "--quiet"],
               cwd=ROOT, check=True, timeout=240)
binary = ROOT / "target/debug/hars-native-kernel"
assert binary.is_file(), "Rust kernel missing"
# Prove accidental invocation on the host is rejected before any TCP connect.
denied = subprocess.run([str(binary),"--lab-mqtt-observe","1"],
                        cwd=ROOT,capture_output=True,text=True,timeout=5)
assert denied.returncode!=0 and "isolated namespace" in denied.stderr,     "native MQTT subscriber allowed operation outside network=none"
denied_v5 = subprocess.run([str(binary), "--lab-mqtt5-rap-observe", "3"],
                           cwd=ROOT, capture_output=True, text=True, timeout=5)
assert denied_v5.returncode != 0 and "isolated namespace" in denied_v5.stderr, (
    "MQTT5 RAP subscriber accidentally allowed the isolated Linux lab/host network"
)
print("PASS: MQTT3 and MQTT5 invocations fail closed outside isolated Docker namespace")
(ROOT / "runtime").mkdir(mode=0o700, exist_ok=True)

def cli(*args: str, timeout: int = 15) -> subprocess.CompletedProcess:
    result = subprocess.run(["docker", *args], cwd=ROOT,
                            capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise RuntimeError(f"isolated lab command failed: {args[:3]} status={result.returncode}: {result.stderr[:600]}")
    return result

def run_rust(name: str, *args: str, timeout: int = 18) -> dict:
    proc = cli("exec", "--user", f"{os.geteuid()}:{os.getegid()}", name, "/opt/hars-native-kernel",
               *args, timeout=timeout)
    return json.loads(proc.stdout)

with tempfile.TemporaryDirectory(prefix="hars-p2b-mqtt-", dir=ROOT / "runtime") as folder:
    temp = Path(folder)
    os.chmod(temp,0o700)
    fixture = {
        "homeassistant/device/kitchen/config": {
            "dev": {"ids": "synthetic-kitchen", "name": "Isolated Lab Kitchen"},
            "o": {"name": "hars-mqtt-test"},
            "cmps": {
                "temp": {"p": "sensor", "unique_id": "lab_unique_temp"},
                "motion": {"p": "binary_sensor", "unique_id": "lab_unique_motion"}
            },
            "state_topic": "lab/readonly/sensor"
        },
        "homeassistant/sensor/bedroom/temp/config": {
            "unique_id": "lab_unique_bedroom",
            "state_topic": "lab/readonly/bedroom"
        },
        # Unrecognized actuator config must be REJECTED by the Rust parser.
        "homeassistant/light/lamp/config": {
            "unique_id": "lab_fake_actuator", "state_topic": "lab/readonly/lamp"
        }
    }
    entries=[]
    for idx,(topic,payload) in enumerate(fixture.items()):
        file=temp/f"payload-{idx}.json"
        file.write_text(json.dumps(payload),encoding="utf-8")
        os.chmod(file,0o600)
        entries.append((topic,file))
    name=f"hars-mqtt-p2b-{os.getpid()}"
    run=[
        "run","-d","--name",name,
        "--network","none","--read-only",
        "--user",f"{os.geteuid()}:{os.getegid()}",
        "--cap-drop","ALL","--security-opt","no-new-privileges",
        "--pids-limit","128","--memory","512m","--cpus","2",
        "--tmpfs","/tmp:rw,nosuid,nodev,size=16m",
        "--mount",f"type=bind,src={binary},dst=/opt/hars-native-kernel,readonly",
        "--mount",f"type=bind,src={temp},dst=/fixtures,readonly",
        IMAGE,
    ]
    try:
        cli(*run,timeout=25)
        inspection=json.loads(cli("inspect",name).stdout)[0]
        host=inspection["HostConfig"]
        assert host["NetworkMode"]=="none"
        assert not inspection["NetworkSettings"].get("Ports"), "MQTT broker port exposed!"
        assert not host["Privileged"]
        assert host["ReadonlyRootfs"]
        assert "ALL" in host["CapDrop"]
        assert host["SecurityOpt"] and "no-new-privileges" in host["SecurityOpt"]
        assert not inspection["Mounts"][0].get("RW")
        interface=cli("exec",name,"sh","-c","cat /proc/self/net/dev; cat /proc/net/route").stdout
        fields=interface.strip().splitlines()
        interfaces=[line.split(":",1)[0].strip() for line in fields[2:] if ":" in line]
        assert interfaces==["lo"],f"Unexpected network interfaces: {interfaces}"
        assert fields[-1].startswith("Iface"), "Unexpected external route in isolated namespace"
        print("PASS: Docker network=none, loopback-only interface, no routes, no exposed ports or capabilities")

        for idx,(topic,_) in enumerate(entries):
            result=run_rust(name,"--lab-mqtt-publish",topic,f"/fixtures/payload-{idx}.json","true")
            assert result["published"] and result["synthetic"] and not result["production_ready"]
        print("PASS: 3 QoS1 retained publish acknowledgements from independent Rust publisher processes")

        for cycle in (1,2):
            data=run_rust(name,"--lab-mqtt-observe","3")
            assert data["received"]==3 and data["retained"]==3
            assert data["previews"]==2 and data["rejected"]==1
            assert data["registry_entries"]==2
            assert data["moli_registered_devices"]==3 and data["moli_native_adapter"]
            assert data["moli_states"]==0, "discovery must not fabricate sensor values"
            assert data["mqtt_broker_isolated"] and data["read_only"]
            assert not data["production_ready"]
            print(f"PASS: Rust subscriber cycle {cycle} sees retained replay, 2 sensor-only configs, rejects actuator")

        # P2e: a separate native Rust subscriber really subscribes to
        # the exact validated sensor state topics after MQTT discovery.
        # No state is fabricated by discovery alone and no actuator can run.
        sensor_payloads = (("lab/readonly/sensor", "ON"),
                           ("lab/readonly/bedroom", "22.2"))
        for idx, (_, value) in enumerate(sensor_payloads):
            f = temp / f"sensor-state-{idx}.txt"
            f.write_text(value, encoding="ascii")
            os.chmod(f, 0o600)
        state_observer = subprocess.Popen(
            ["docker", "exec", "--user", f"{os.geteuid()}:{os.getegid()}",
             name, "/opt/hars-native-kernel", "--lab-mqtt-sensor-observe", "3", "2"],
            cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        )
        try:
            ready, _, _ = select.select([state_observer.stdout], [], [], 10)
            assert ready, "Rust sensor state subscriber did not confirm real SUBACKs"
            assert state_observer.stdout.readline().strip() == "LAB_MQTT_STATES_READY"
            for idx, (topic, _) in enumerate(sensor_payloads):
                assert run_rust(name, "--lab-mqtt-publish", topic,
                                f"/fixtures/sensor-state-{idx}.txt", "false")["published"]
            output, error = state_observer.communicate(timeout=16)
            assert state_observer.returncode == 0, f"native MQTT state stream failed: {error[:500]}"
            states = json.loads(output.strip().splitlines()[-1])
            assert states["received"] == 3 and states["state_packets"] == 2
            assert states["moli_registered_devices"] == 3
            assert states["moli_states"] == 3 and states["moli_binary_true"] == 1
            assert states["moli_native_adapter"] and states["read_only"]
            assert not states["production_ready"]
            print("PASS: two real MQTT sensor-topic publishes updated 3 typed Moli states; read-only")
        finally:
            if state_observer.poll() is None:
                state_observer.kill()
                state_observer.wait(timeout=5)
            if state_observer.stdout:
                state_observer.stdout.close()
            if state_observer.stderr:
                state_observer.stderr.close()

        # A retained state publication is explicitly outside the test
        # publisher's contract; we do not silently pretend it is an update.
        denied_state = subprocess.run(
            ["docker", "exec", "--user", f"{os.geteuid()}:{os.getegid()}",
             name, "/opt/hars-native-kernel", "--lab-mqtt-publish",
             "lab/readonly/bedroom", "/fixtures/sensor-state-1.txt", "true"],
            cwd=ROOT, capture_output=True, text=True, timeout=8,
        )
        assert denied_state.returncode != 0, "unsafe retained sensor state was published"
        print("PASS: lab state publisher refuses retained state and never owns actuators")

        # P2f RED→GREEN: real MQTT 5 subscriber with RAP=1 must preserve
        # live RETAIN and distinguish a transient empty PUBLISH from
        # a broker-retained tombstone. The published payloads are fictitious.
        mqtt5_topic = "homeassistant/sensor/p2f_probe/config"
        mqtt5_config = temp / "mqtt5-rap-probe.json"
        mqtt5_config.write_text(json.dumps({
            "unique_id": "lab_p2f_probe",
            "state_topic": "lab/readonly/p2f"
        }), encoding="utf-8")
        os.chmod(mqtt5_config, 0o600)
        empty_mqtt5 = temp / "mqtt5-empty"
        empty_mqtt5.write_bytes(b"")
        os.chmod(empty_mqtt5, 0o600)
        rap_observer = subprocess.Popen(
            ["docker", "exec", "--user", f"{os.geteuid()}:{os.getegid()}",
             name, "/opt/hars-native-kernel", "--lab-mqtt5-rap-observe", "3"],
            cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        )
        try:
            ready, _, _ = select.select([rap_observer.stdout], [], [], 10)
            assert ready and rap_observer.stdout.readline().strip() == "LAB_MQTT5_RAP_READY", (
                "native MQTT5 RAP observer did not subscribe successfully"
            )
            for data_file, retain in [
                ("mqtt5-empty", "false"),
                ("mqtt5-rap-probe.json", "true"),
                ("mqtt5-empty", "true"),
            ]:
                assert run_rust(name, "--lab-mqtt-publish", mqtt5_topic,
                                f"/fixtures/{data_file}", retain)["published"]
            output, error = rap_observer.communicate(timeout=16)
            assert rap_observer.returncode == 0, f"MQTT5 observer failed: {error[:500]}"
            rap = json.loads(output.strip().splitlines()[-1])
            assert rap["received"] == 3
            assert rap["ignored_empty_nonretained"] == 1
            assert rap["validated_retained_configs"] == 1
            assert rap["validated_retained_deletes"] == 1
            assert rap["mqtt5_rap"] and rap["read_only"] and not rap["production_ready"]
            print("PASS: MQTT5 RAP live empty nonretained ignored; retained update and real tombstone distinguished")
        finally:
            if rap_observer.poll() is None:
                rap_observer.kill()
                rap_observer.wait(timeout=5)
            if rap_observer.stdout:
                rap_observer.stdout.close()
            if rap_observer.stderr:
                rap_observer.stderr.close()

        # P2d LIVE proof: MQTT 3.1.1 MUST clear RETAIN on delivery to an
        # existing subscription, including an actual retained tombstone.
        # The native registry must not mistake this for a verified deletion.
        live_empty = temp / "live-tombstone-empty"
        live_empty.write_bytes(b"")
        os.chmod(live_empty, 0o600)
        observer = subprocess.Popen(
            ["docker", "exec", "--user", f"{os.geteuid()}:{os.getegid()}",
             name, "/opt/hars-native-kernel", "--lab-mqtt-observe-ready", "4"],
            cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        )
        try:
            ready, _, _ = select.select([observer.stdout], [], [], 10)
            assert ready, "isolated Rust MQTT subscriber did not acknowledge subscription"
            assert observer.stdout.readline().strip() == "LAB_MQTT_SUBSCRIBED"
            assert run_rust(name, "--lab-mqtt-publish", entries[1][0],
                            "/fixtures/live-tombstone-empty", "true")["published"]
            output, error = observer.communicate(timeout=16)
            assert observer.returncode == 0, f"live MQTT observer failed: {error[:350]}"
            live = json.loads(output.strip().splitlines()[-1])
            assert live["received"] == 4 and live["retained"] == 3
            assert live["previews"] == 2 and live["rejected"] == 2
            assert live["removes"] == 0 and live["registry_entries"] == 2
            assert live["moli_registered_devices"] == 3 and live["read_only"]
            print("PASS: live retained tombstone delivered RETAIN=0; native Moli registry refuses ambiguous delete")
        finally:
            if observer.poll() is None:
                observer.kill()
                observer.wait(timeout=5)
            if observer.stdout:
                observer.stdout.close()
            if observer.stderr:
                observer.stderr.close()
        assert run_rust(name, "--lab-mqtt-publish", entries[1][0],
                        "/fixtures/payload-1.json", "true")["published"]
        print("PASS: isolated retained fixture restored after live-only test")

        # A QoS1 live-only message is NOT stored by the broker.
        nonretained=temp/"only-live.json"
        nonretained.write_text(json.dumps({
            "unique_id":"lab_live_only","state_topic":"lab/readonly/live"
        }))
        os.chmod(nonretained,0o600)
        result=run_rust(name,"--lab-mqtt-publish",
                        "homeassistant/sensor/live/config",
                        "/fixtures/only-live.json","false")
        assert result["published"]
        again=run_rust(name,"--lab-mqtt-observe","3")
        assert again["received"]==3 and again["previews"]==2
        print("PASS: non-retained publication not resurrected on subscriber reconnect")

        # OASIS MQTT 3.1.1: RETAIN=true is only exposed on NEW
        # subscriptions. Validate migration/deletion by authoritative
        # fresh retained replay, NOT by trusting live PUBLISH retain flags.
        migration = temp / "migrate.json"
        migration.write_text('{"migrate_discovery":true}',encoding="utf-8")
        os.chmod(migration,0o600)
        migration_topic = entries[1][0]
        result=run_rust(name,"--lab-mqtt-publish",
                        migration_topic,"/fixtures/migrate.json","true")
        assert result["published"]
        migrated=run_rust(name,"--lab-mqtt-observe","3")
        assert migrated["migrations"]==1 and migrated["retained"]==3
        assert migrated["previews"]==1 and migrated["rejected"]==1
        assert migrated["registry_entries"]==1
        assert migrated["moli_registered_devices"]==2 and migrated["read_only"]
        print("PASS: actual retained MQTT migration seen on fresh subscription, only 2 sensor points remain")

        empty = temp / "empty.json"
        empty.write_bytes(b"")
        os.chmod(empty,0o600)
        result=run_rust(name,"--lab-mqtt-publish",
                        migration_topic,"/fixtures/empty.json","true")
        assert result["published"]
        after_delete=run_rust(name,"--lab-mqtt-observe","2")
        assert after_delete["received"]==2 and after_delete["retained"]==2
        assert after_delete["migrations"]==0 and after_delete["previews"]==1
        assert after_delete["registry_entries"]==1
        assert after_delete["moli_registered_devices"]==2
        print("PASS: real broker erased retained migration topic, no resurrection after subscriber restart")

        # Final cleanup also tests removal of a 2-sensor device config.
        result=run_rust(name,"--lab-mqtt-publish",
                        entries[0][0],"/fixtures/empty.json","true")
        assert result["published"]
        after_device_delete=run_rust(name,"--lab-mqtt-observe","1")
        assert after_device_delete["received"]==1 and after_device_delete["rejected"]==1
        assert after_device_delete["registry_entries"]==0
        assert after_device_delete["moli_registered_devices"]==0
        assert after_device_delete["read_only"] and not after_device_delete["production_ready"]
        print("PASS: real broker clears 2-sensor device discovery and registry stays empty on reconnect")
    finally:
        subprocess.run(["docker","rm","-f",name],cwd=ROOT,stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL,timeout=15)
        if subprocess.run(["docker","ps","-a","--format","{{.Names}}"],
                          cwd=ROOT,capture_output=True,text=True).stdout.find(name)!=-1:
            raise AssertionError("Lab MQTT container leaked")

print("PASS: isolated Mosquitto container/fixtures removed; no broker exposed to host or LAN")
