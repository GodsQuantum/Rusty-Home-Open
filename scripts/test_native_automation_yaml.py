#!/usr/bin/env python3
"""P3a native Rust HA-YAML compiler dry-run, separate process, no active IO."""
from __future__ import annotations
import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
if os.geteuid() == 0 or not (
    os.uname().nodename == "dev-build" or os.environ.get("GITHUB_ACTIONS") == "true"
):
    raise SystemExit("P3a requires unprivileged isolated Linux lab or CI")
subprocess.run(["cargo", "build", "--locked", "--quiet", "-p", "hars-native-kernel"],
               cwd=ROOT, check=True, timeout=240)
BINARY = ROOT / "target/debug/hars-native-kernel"
runtime = ROOT / "runtime"
runtime.mkdir(mode=0o700, exist_ok=True)

modern = """id: "lab_light_automation"
mode: single
triggers:
  - trigger: state
    entity_id: binary_sensor.lab_motion
    from: "off"
    to: "on"
conditions:
  - condition: state
    entity_id: light.lab_room
    state: "off"
actions:
  - action: light.turn_on
    target:
      entity_id: light.lab_room
"""
legacy = modern.replace("triggers:", "trigger:").replace("- trigger: state", "- platform: state").replace("conditions:", "condition:").replace("actions:", "action:").replace("- action: light.turn_on", "- service: light.turn_on")

with tempfile.TemporaryDirectory(prefix="hars-p3a-", dir=runtime) as path:
    tmp = Path(path)
    tmp.chmod(0o700)
    def source(name: str, content: str) -> Path:
        file = tmp / name
        file.write_text(content, encoding="utf8")
        file.chmod(0o600)
        return file
    def run(src: Path, previous: str, next_state: str, condition: str, event_entity="binary_sensor.lab_motion"):
        return subprocess.run([str(BINARY), "--lab-ha-automation-yaml", str(src),
            event_entity, previous, next_state, condition], cwd=ROOT,
            capture_output=True, text=True, timeout=8)
    plans = []
    for variant, content in [("modern", modern), ("legacy", legacy)]:
        file = source(f"{variant}.yaml", content)
        p = run(file, "off", "on", "off")
        assert p.returncode == 0, f"{variant}: {p.stderr[:500]}"
        report = json.loads(p.stdout)
        assert report["simulation"] == {
            "triggered": True,
            "proposed_action": "light.turn_on:light.lab_room",
            "read_only": True, "production_ready": False,
        }, report
        plans.append(report["plan"])
        for old, new, condition in [("on", "on", "off"), ("off", "off", "off"),
                                    ("off", "on", "on"), ("unknown", "on", "off")]:
            negative = run(file, old, new, condition)
            assert negative.returncode == 0, negative.stderr[:350]
            silent = json.loads(negative.stdout)["simulation"]
            assert not silent["triggered"] and silent["proposed_action"] is None
    assert plans[0] == plans[1]
    spoofed = run(source("modern2.yaml", modern), "off", "on", "off", "binary_sensor.lab_other")
    assert spoofed.returncode == 0 and not json.loads(spoofed.stdout)["simulation"]["triggered"]
    print("PASS: native Rust modern/legacy HA YAML plans and dry-run transitions/conditions")

    for i, evil in enumerate([
        modern.replace("light.lab_room", "light.actual_home"),
        modern.replace("light.turn_on", "shell_command.execute"),
        modern.replace("triggers:", "trigger:") + "\ntriggers: []",
        modern + "\nunknown_key: 2",
        modern.replace('from: "off"', 'from: "{{ states(\"light.real\") }}"'),
        modern.replace("mode: single", "mode: parallel"),
        modern.replace('from: "off"', 'from: &inject off'),
        modern + "\nactions: []",
    ]):
        p = run(source(f"bad-{i}.yaml", evil), "off", "on", "off")
        assert p.returncode != 0, f"unsafe source unexpectedly parsed {i}"
    outside = run(Path("/etc/passwd"), "off", "on", "off")
    assert outside.returncode != 0
    assert stat.S_IMODE(source("empty.yaml", "").stat().st_mode) == 0o600
    print("PASS: nonlab targets, shell commands, templates, anchors, duplicate keys and outside paths rejected")
print("P3a PASS: isolated temp YAML fixtures cleaned; no commands, ports or devices touched")
