#!/usr/bin/env python3
"""P2i: genuine isolated Mosquitto SIGKILL; persisted MQTT5 discovery is NOT authority.

No HAOS, external network, radio, production MQTT, secrets or device writes.
"""
from __future__ import annotations

import json
import os
from pathlib import Path
import select
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent
IMAGE = "hars-mqtt-lab:20261019"
if os.geteuid() == 0 or not (
    os.uname().nodename == "dev-build" or os.environ.get("GITHUB_ACTIONS") == "true"
):
    raise SystemExit("P2i requires unprivileged DEV lab or GitHub CI")

def run(*args: str, timeout: int = 35) -> str:
    result = subprocess.run(args, cwd=ROOT, text=True, capture_output=True, timeout=timeout)
    if result.returncode:
        raise RuntimeError(f"{args[:4]} failed ({result.returncode}): {result.stderr[:650]}")
    return result.stdout

run("cargo", "build", "--locked", "--quiet", "-p", "hars-native-kernel", timeout=240)
binary = ROOT / "target/debug/hars-native-kernel"
denied = subprocess.run(
    [str(binary), "--lab-mqtt5-registry-observe", "/state", "0"],
    cwd=ROOT, text=True, capture_output=True, timeout=5,
)
assert denied.returncode and "isolated namespace" in denied.stderr
run("docker", "image", "inspect", IMAGE)

runtime = ROOT / "runtime"
runtime.mkdir(mode=0o700, exist_ok=True)
with tempfile.TemporaryDirectory(prefix="hars-p2i-", dir=runtime) as tmp:
    folder = Path(tmp)
    folder.chmod(0o700)
    db = folder / "registry"
    db.mkdir(mode=0o700)
    conf = folder / "config.json"
    conf.write_text(json.dumps({
        "unique_id": "lab_p2h_probe", "state_topic": "lab/readonly/p2h"
    }))
    sensor = folder / "state.txt"
    sensor.write_text("21.5")
    newer = folder / "new_state.txt"
    newer.write_text("31.2")
    removed = folder / "tombstone.txt"
    removed.write_bytes(b"")
    for file in (conf, sensor, newer, removed):
        file.chmod(0o600)
    name = f"hars-p2i-{os.getpid()}"
    user = f"{os.geteuid()}:{os.getegid()}"
    docker = lambda *args, timeout=35: run("docker", *args, timeout=timeout)
    try:
        docker(
            "run", "-d", "--name", name, "--network", "none", "--read-only",
            "--user", user, "--cap-drop", "ALL",
            "--security-opt", "no-new-privileges",
            "--pids-limit", "128", "--memory", "512m", "--cpus", "2",
            "--tmpfs", "/tmp:rw,nosuid,nodev,size=16m",
            "--mount", f"type=bind,src={binary},dst=/opt/hars-native-kernel,readonly",
            "--mount", f"type=bind,src={folder},dst=/fixtures,readonly",
            "--mount", f"type=bind,src={db},dst=/state",
            IMAGE,
        )
        info = json.loads(docker("inspect", name))[0]
        host = info["HostConfig"]
        assert host["NetworkMode"] == "none" and host["ReadonlyRootfs"]
        assert not host["Privileged"] and "ALL" in host["CapDrop"]
        assert not info["NetworkSettings"].get("Ports")
        assert all(not m["RW"] or m["Destination"] == "/state" for m in info["Mounts"])

        def publish(path: str, topic: str, retained: bool) -> None:
            data = json.loads(docker(
                "exec", "--user", user, name, "/opt/hars-native-kernel",
                "--lab-mqtt-publish", topic, f"/fixtures/{path}",
                str(retained).lower(),
            ))
            assert data["published"] and not data["production_ready"]

        def spawn(expected: int, *, readd: bool = False) -> subprocess.Popen:
            op = "--lab-mqtt5-registry-readd" if readd else "--lab-mqtt5-registry-observe"
            p = subprocess.Popen(
                ["docker", "exec", "--user", user, name, "/opt/hars-native-kernel",
                 op, "/state", str(expected)],
                cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            )
            ready, _, _ = select.select([p.stdout], [], [], 10)
            assert ready and p.stdout.readline().strip() == "LAB_MQTT5_REGISTRY_READY", (
                "real MQTT5 SUBACK not reached"
            )
            return p

        def finish(p: subprocess.Popen, *, failure: str | None = None) -> dict:
            try:
                out, err = p.communicate(timeout=17)
                if failure is not None:
                    assert p.returncode != 0 and failure in err, (
                        f"expected {failure!r}, got {p.returncode}: {err[:450]}"
                    )
                    return {}
                assert p.returncode == 0, f"subscriber exited {p.returncode}: {err[:650]}"
                report = json.loads(out.strip().splitlines()[-1])
                assert report["read_only"] and not report["production_ready"]
                assert not report["device_ownership_granted"]
                assert not report["broker_snapshot_authoritative"]
                return report
            finally:
                if p.poll() is None:
                    p.kill()
                    p.wait(timeout=5)
                if p.stdout:
                    p.stdout.close()
                if p.stderr:
                    p.stderr.close()

        topic = "homeassistant/sensor/p2h_probe/config"
        publish("config.json", topic, True)
        first = finish(spawn(1))
        assert first["retained_configs"] == first["durable_entries"] == 1
        assert first["moli_devices"] == 1 and first["recovered_stale_on_open"] == 0
        print("P2I_INITIAL_REAL_BROKER_CONFIG_DURABLE_AND_MOLI_PASS", flush=True)

        in_flight = spawn(2)
        assert in_flight.poll() is None
        docker("kill", "--signal", "KILL", name)
        stopped = json.loads(docker("inspect", name))[0]["State"]
        assert not stopped["Running"] and stopped["ExitCode"] == 137, stopped
        out, err = in_flight.communicate(timeout=10)
        assert in_flight.returncode != 0 and "production_ready" not in out, (
            f"subscriber unexpectedly survived broker SIGKILL: {out[:350]} {err[:350]}"
        )
        in_flight.stdout.close()
        in_flight.stderr.close()
        print("P2I_REAL_MOSQUITTO_SIGKILL_TERMINATES_ACTIVE_SUBSCRIBER_PASS", flush=True)

        docker("start", name)
        time.sleep(0.5)
        after = finish(spawn(0))
        assert after["recovered_stale_on_open"] == 1
        assert after["durable_entries"] == 1 and after["moli_devices"] == 0
        assert after["received"] == 0 and after["moli_states"] == 0
        print("P2I_RESTART_EMPTY_BROKER_STALE_REDB_ZERO_MOLI_PASS", flush=True)

        refused = spawn(1)
        publish("state.txt", "lab/readonly/p2h", False)
        finish(refused, failure="unverified, retained, or unsafe MQTT5 sensor state")
        print("P2I_STALE_REDB_CANNOT_AUTHORIZE_UNSOLICITED_SENSOR_STATE_PASS", flush=True)

        publish("config.json", topic, True)
        recovered = spawn(2)
        publish("state.txt", "lab/readonly/p2h", False)
        final = finish(recovered)
        assert final["recovered_stale_on_open"] == 1
        assert final["retained_configs"] == final["sensor_packets"] == 1
        assert final["durable_entries"] == final["moli_devices"] == final["moli_states"] == 1
        print("P2I_FRESH_BROKER_REATTESTATION_RECOVERS_READONLY_MOLI_PASS", flush=True)

        # NEW independent P2k scenario: broker stays alive and the SAME Moli
        # process observes config -> state -> tombstone -> SAME-ID config ->
        # new state. No false claim from just P2j in-memory event testing.
        p2k = spawn(5, readd=True)
        publish("state.txt", "lab/readonly/p2h", False)
        publish("tombstone.txt", topic, True)
        publish("config.json", topic, True)
        publish("new_state.txt", "lab/readonly/p2h", False)
        readd = finish(p2k)
        assert readd["received"] == 5
        assert readd["retained_configs"] == 2
        assert readd["retained_tombstones"] == 1
        assert readd["sensor_packets"] == 2
        assert readd["old_state_cleared_before_readd"]
        assert readd["same_id_readd_verified"]
        assert readd["durable_entries"] == readd["moli_devices"] == readd["moli_states"] == 1
        print("P2K_GENUINE_MQTT5_BROKER_RETAINED_DELETE_READD_SAME_MOLI_ID_PASS", flush=True)
        print("P2K_NATIVE_MOLI_OLD_STATE_CLEARED_NEW_VALUE_READONLY_PASS", flush=True)
    finally:
        subprocess.run(["docker", "rm", "-f", name], cwd=ROOT,
                       capture_output=True, text=True, timeout=20)

print("P2I_ISOLATED_BROKER_CRASH_RECOVERY_COMPLETE_NO_PRODUCTION_ACCESS_PASS")
