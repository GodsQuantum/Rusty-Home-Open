#!/usr/bin/env python3
"""Scoped ASan target for the newly exposed disabled-only HAOS YAML parser."""
from pathlib import Path
import json
import tomllib

root=Path(__file__).resolve().parent.parent
target=(root/"fuzz/fuzz_targets/disabled_haos_automation.rs").read_text()
cargo=tomllib.loads((root/"fuzz/Cargo.toml").read_text())
workflow=(root/".github/workflows/fuzz.yml").read_text()
assert any(x["name"]=="disabled_haos_automation" for x in cargo["bin"])
assert cargo["dependencies"]["hars-native-kernel"]["path"]=="../crates/native-kernel"
assert "parse_disabled_backup_plan(yaml)" in target
assert "parse_disabled_backup_plans(yaml)" in target
assert "plans.len() <= 8" in target
assert "preview.read_only && !preview.production_ready" in target
assert "compile(&plan)" in target
assert "graph.nodes.len(), 3" in target
assert "if raw.len() > 8192" in target
for denied in ("TcpListener","TcpStream","SUPERVISOR_TOKEN","VM100",
               "hub.command(", "service_call", "Origin::Ui"):
    assert denied not in target,denied
for required in ("fuzz run disabled_haos_automation",
                 "-max_total_time=45","-max_total_time=180",
                 "fuzz/artifacts/disabled_haos_automation/",
                 "scripts/test_p15_disabled_automation_fuzz_contract.py",
                 "crates/native-kernel/src/automation_yaml.rs"):
    assert required in workflow,required
corpus=root/"fuzz/corpus/disabled_haos_automation"
files=list(corpus.glob("*.yaml"))
assert len(files)==8
for p in files:
    raw=p.read_bytes()
    assert 30<len(raw)<8192
    assert b"SUPERVISOR_TOKEN" not in raw and b"real-person" not in raw
assert b"initial_state: false" in (corpus/"disabled-synthetic.yaml").read_bytes()
assert b"initial_state: true" in (corpus/"enabled-denied.yaml").read_bytes()
assert b"{{ banned }}" in (corpus/"template-denied.yaml").read_bytes()
assert b"p8ck_second_lab_door" in (corpus/"double-disabled-synthetic.yaml").read_bytes()
assert b"initial_state: true" in (corpus/"mixed-enabled-denied.yaml").read_bytes()
assert "disabled_haos_automation/*" in (root/"fuzz/.gitignore").read_text()
gates=json.loads((root/"config/release-gates.json").read_text())
assert next(x for x in gates["required_gates"] if x["id"]=="security_stress_fuzz")["status"] != "certified"
print("P15_DISABLED_HAOS_AUTOMATION_FUZZ_SCOPE_AND_SYNTHETIC_SEEDS_PASS")
