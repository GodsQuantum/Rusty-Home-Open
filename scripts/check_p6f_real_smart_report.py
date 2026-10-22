#!/usr/bin/env python3
"""Fail-closed genuine Smart Announcements 0.8.3 Core lifecycle report."""
import json
from pathlib import Path
import sys
REQUIRED={
    "upstream":"mike-nott/smart-announcements",
    "version":"0.8.3",
    "commit":"e5a843be497e5591233896035a4a20fd60e939bb",
    "core_real_config_flow":True,
    "required_inputs_validated":True,
    "fake_person_and_area":True,
    "core_loaded":True,
    "core_unloaded_and_reloaded":True,
    "core_removed":True,
    "duplicate_rejected":True,
    "synthetic_tts_not_executed":True,
    "real_user_devices_tested":False,
    "production_ready":False,
}
def verify(report):
    if not isinstance(report,dict) or set(report)!=set(REQUIRED):
        raise ValueError("unexpected HA Core Smart Announcements evidence schema")
    for field,expected in REQUIRED.items():
        actual=report[field]
        if type(actual) is not type(expected) or actual!=expected:
            raise ValueError("invalid Core evidence: "+field)
def main():
    if len(sys.argv)!=2:
        raise SystemExit("usage: one genuine HA Core report path")
    p=Path(sys.argv[1])
    if not p.is_file() or p.stat().st_size>20000:
        raise SystemExit("missing/oversized genuine Core proof")
    try:verify(json.loads(p.read_text()))
    except (ValueError,UnicodeError,OSError) as exc:
        raise SystemExit("P6F_CORE_FAIL: "+str(exc)) from None
    print("P6F_REAL_SMART_ANNOUNCEMENTS_CONFIG_FLOW_LIFECYCLE_PASS")
    print("Synthetic flow only; no physical speakers, TTS service or production integration certified")
if __name__=="__main__":
    main()
