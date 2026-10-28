#!/usr/bin/env python3
"""P8ec opt-in Core matrix guard and strict synthetic proof mutations."""
import json
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parent))
from check_p6d_real_watchman_report import EXPECTED as BASE,verify as verify_old
from check_p8ec_watchman_current_core import EXPECTED as CURRENT,verify as verify_new
p=Path(__file__).resolve().parents[1]
sh=(p/"scripts/run_p6d_real_watchman_core.sh").read_text()
py=(p/"scripts/test_p6d_real_watchman_core.py").read_text()
workflow=(p/".github/workflows/p8ec-watchman-current-core.yml").read_text()
assert "HARS_CURRENT_CORE_HACS_LAB" in sh
assert 'EXPECTED_CORE_VERSION="2026.10.2"' in sh
assert 'EXPECTED_CORE_VERSION="2026.10.4"' in sh
assert 'HARS_EXPECTED_CORE_VERSION' in sh
assert 'candidate["oci_index"]' in sh and 'pins["stable_candidate"]' in sh
assert "ghcr.io/home-assistant/home-assistant@sha256:33afb0659d786213264c2211f8d91c5fe87461c56eba6a3a546eb05fc71e89f4" in sh
assert 'if [[ "$EXPECTED_CORE_VERSION" == "2026.10.4" ]]' in sh
assert 'from homeassistant.const import __version__ as ha_core_version' in py
assert 'ha_core_version == expected_core' in py
assert 'proof["core_version"] = ha_core_version' in py
assert 'HARS_CURRENT_CORE_HACS_LAB: "2026.10.4"' in workflow
assert "--network none" in sh
assert "--read-only" in sh
assert "no-new-privileges" in sh
assert "GITHUB_ACTIONS" in sh
assert all(tag not in sh+workflow for tag in ("192.168.","SUPERVISOR_TOKEN","vm100","qm stop","pct exec","workflow_dispatch:"))
verify_old(BASE.copy()) # previous 2026.10.2 contract unchanged
data=CURRENT.copy()
verify_new(data)
for key,value in [("core_version","2026.10.2"),
                  ("core_version","2026.11.0dev0"),
                  ("genuine_core_remove",False),
                  ("production_ready",True)]:
    fake=data.copy()
    fake[key]=value
    try:
        verify_new(fake)
    except ValueError:
        pass
    else:
        raise AssertionError("forged candidate proof accepted: "+key)
for bad in ({**data,"injected":True},{k:v for k,v in data.items() if k!="core_version"}):
    try:
        verify_new(bad)
    except ValueError:
        pass
    else:
        raise AssertionError("wrong candidate shape accepted")
print("P8EC_CORE_CURRENT_EXACT_PIN_LAB_SCOPE_AND_FORGED_PROOF_REFUSAL_PASS")
