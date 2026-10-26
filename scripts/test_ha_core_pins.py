#!/usr/bin/env python3
"""Fail closed if any runnable Home Assistant Core lab is silently outdated."""
import json
import re
from pathlib import Path

root=Path(__file__).resolve().parent.parent
pins=json.loads((root/"config/ha-core-pins.json").read_text())
stable=pins["stable"]; dev=pins["upcoming_dev"]
assert stable["version"]=="2026.10.2"
candidate=pins["stable_candidate"]
assert candidate=={
 "version":"2026.10.4",
 "oci_index":"sha256:23f4dfbb69ce76315dbc4de7915504f1c2dcecd7ca2648113a493a385972e133",
 "qualification":"pending_disposable_real_core_ci",
}
assert candidate["oci_index"]!=stable["oci_index"]
assert dev["version"].startswith("2026.11.")
assert re.fullmatch(r"sha256:[0-9a-f]{64}",stable["oci_index"])
assert re.fullmatch(r"sha256:[0-9a-f]{64}",dev["oci_index"])

all_references={}
for path in (root/"scripts").iterdir():
 if path.suffix not in (".sh",".py") or path.name=="test_ha_core_pins.py": continue
 contents=path.read_text()
 matches=re.findall(r"ghcr[.]io/home-assistant/home-assistant@(sha256:[a-f0-9]{64})",contents)
 if matches:all_references[path.name]=matches
assert len(all_references)>=12,all_references
for path,refs in all_references.items():
 assert all(ref==stable["oci_index"] for ref in refs),(path,refs,stable)
workflow=(root/".github/workflows/core-interop-public.yml").read_text()
assert "Genuine Core 2026.10.2" in workflow
assert "2026.10.1" not in workflow
marketplace=(root/".github/workflows/marketplace-abi.yml").read_text()
assert dev["oci_index"] in marketplace
assert stable["oci_index"] in marketplace
assert (root/"docs/MARKETPLACE_CORE_2026_11_ARCHITECTURE.md").is_file()
print("PASS: ",len(all_references),"real-Core lab scripts pinned to official stable",stable["version"])
assert "HARS_CURRENT_CORE_LAB" in (root/"scripts/run_config_entry_lifecycle_core.sh").read_text()
assert "HARS_EXPECTED_CORE_VERSION" in (root/"scripts/test_config_entry_lifecycle_core.py").read_text()
print("PASS: official 2026.10.4 candidate separately pinned with strict runtime version and original baseline unchanged")
print("PASS: upcoming Marketplace dev image pinned independently, no version mixing")
