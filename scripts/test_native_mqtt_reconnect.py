#!/usr/bin/env python3
"""P2h cross-process MQTT5 + redb + Moli. Real isolated Mosquitto, no HAOS."""
from __future__ import annotations
import json
import os
from pathlib import Path
import select
import subprocess
import tempfile
ROOT = Path(__file__).resolve().parent.parent
IMAGE = "hars-mqtt-lab:20261019"
if os.geteuid() == 0 or not (os.uname().nodename == "dev-build" or os.environ.get("GITHUB_ACTIONS") == "true"):
    raise SystemExit("P2h requires unprivileged LXC700 or GitHub CI")
subprocess.run(["cargo", "build", "--locked", "--quiet", "-p", "hars-native-kernel"],
               cwd=ROOT, check=True, timeout=240)
binary = ROOT / "target/debug/hars-native-kernel"
neg = subprocess.run([str(binary), "--lab-mqtt5-registry-observe", "/state", "0"],
                     capture_output=True, text=True, cwd=ROOT, timeout=5)
assert neg.returncode != 0 and "isolated namespace" in neg.stderr, "Host networking not refused"
runtime = ROOT / "runtime"
runtime.mkdir(mode=0o700, exist_ok=True)

def docker(*argv: str, timeout: int = 30) -> str:
    p = subprocess.run(["docker", *argv], cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    if p.returncode:
        raise RuntimeError(f"docker {argv[:3]} returned {p.returncode}: {p.stderr[:350]}")
    return p.stdout

with tempfile.TemporaryDirectory(prefix="hars-p2h-", dir=runtime) as dirpath:
    folder = Path(dirpath)
    os.chmod(folder, 0o700)
    db = folder / "registry"
    db.mkdir(mode=0o700)
    os.chmod(db, 0o700)
    conf = folder / "sensor.json"
    conf.write_text(json.dumps({"unique_id": "lab_p2h_probe", "state_topic": "lab/readonly/p2h"}))
    sensor = folder / "state.txt"
    sensor.write_text("21.5")
    empty = folder / "empty"
    empty.write_bytes(b"")
    for f in (conf, sensor, empty):
        f.chmod(0o600)
    name = f"hars-p2h-{os.getpid()}"
    user = f"{os.geteuid()}:{os.getegid()}"
    cmd = [
        "run", "-d", "--name", name, "--network", "none", "--read-only",
        "--user", user, "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
        "--pids-limit", "128", "--memory", "512m", "--cpus", "2",
        "--tmpfs", "/tmp:rw,nosuid,nodev,size=16m",
        "--mount", f"type=bind,src={binary},dst=/opt/hars-native-kernel,readonly",
        "--mount", f"type=bind,src={folder},dst=/fixtures,readonly",
        "--mount", f"type=bind,src={db},dst=/state",
        IMAGE
    ]
    def publish(path: str, retained: bool) -> None:
        result = json.loads(docker("exec", "--user", user, name,
            "/opt/hars-native-kernel", "--lab-mqtt-publish",
            "homeassistant/sensor/p2h_probe/config" if path != "state.txt" else "lab/readonly/p2h",
            f"/fixtures/{path}", str(retained).lower()))
        assert result["published"] and not result["production_ready"]

    def observer(expected: int, after_ready=None) -> dict:
        p = subprocess.Popen(
            ["docker", "exec", "--user", user, name,
             "/opt/hars-native-kernel", "--lab-mqtt5-registry-observe", "/state", str(expected)],
            cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        try:
            ready, _, _ = select.select([p.stdout], [], [], 10)
            assert ready and p.stdout.readline().strip() == "LAB_MQTT5_REGISTRY_READY", "no MQTT5 SUBACK"
            if after_ready:
                after_ready()
            out, err = p.communicate(timeout=17)
            assert p.returncode == 0, f"observer exited {p.returncode}: {err[:800]}"
            data = json.loads(out.strip().splitlines()[-1])
            assert data["read_only"] and not data["production_ready"]
            assert not data["device_ownership_granted"] and not data["broker_snapshot_authoritative"]
            return data
        finally:
            if p.poll() is None:
                p.kill()
                p.wait(timeout=5)
            if p.stdout:
                p.stdout.close()
            if p.stderr:
                p.stderr.close()
    try:
        docker(*cmd)
        inspect = json.loads(docker("inspect", name))[0]
        assert inspect["HostConfig"]["NetworkMode"] == "none"
        assert inspect["HostConfig"]["ReadonlyRootfs"]
        assert not inspect["HostConfig"]["Privileged"]
        assert not inspect["NetworkSettings"].get("Ports")
        assert all(not m["RW"] or m["Destination"] == "/state" for m in inspect["Mounts"])
        publish("sensor.json", True)
        a = observer(1)
        assert a["retained_configs"] == 1 and a["moli_devices"] == 1
        assert a["durable_entries"] == 1 and a["recovered_stale_on_open"] == 0
        print("PASS: true MQTT5 retained broker replay persisted and projected read-only to Moli")

        publish("empty", True)  # offline tombstone is NOT replayed to new subscribers
        b = observer(0)
        assert b["received"] == 0 and b["recovered_stale_on_open"] == 1
        assert b["durable_entries"] == 1 and b["moli_devices"] == 0
        print("PASS: broker tombstone during downtime cannot resurrect cached stale Moli device")

        publish("sensor.json", True)
        c = observer(2, lambda: publish("state.txt", False))
        assert c["recovered_stale_on_open"] == 1
        assert c["retained_configs"] == 1 and c["sensor_packets"] == 1
        assert c["moli_devices"] == 1 and c["moli_states"] == 1
        print("PASS: reconnect requires fresh retained replay before typed MQTT state enters Moli")

        d = observer(3, lambda: (publish("empty", False), publish("empty", True)))
        assert d["retained_configs"] == 1 and d["ignored_unretained_empty"] == 1
        assert d["retained_tombstones"] == 1 and d["durable_entries"] == 0
        assert d["moli_devices"] == 0
        print("PASS: transient empty ignored, retained tombstone commits deletion and removes Moli device")
    finally:
        try:
            docker("rm", "-f", name, timeout=15)
        except Exception:
            pass
print("P2h PASS: true broker, 4 real subscriber processes, fully isolated, temporary state cleaned")
