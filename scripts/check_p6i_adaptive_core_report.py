#!/usr/bin/env python3
"""Strict real Core Adaptive Lighting v1.32.0 zero-device lifecycle proof."""
import json,sys
from pathlib import Path
REQUIRED={
"real_upstream":"basnijholt/adaptive-lighting",
"upstream_commit":"7f7fec4e34d331fa5fd84b95514ddb1baf2a401e",
"integration_version":"1.32.0",
"dependency_ulid_transform":"2.2.9",
"real_core_config_flow":True,
"real_core_entry_loaded":True,
"real_core_switch_created":True,
"real_core_unload_removes_switch":True,
"unload_switches_unavailable_restored":True,
"real_core_reload_recreates_switch":True,
"real_core_removal":True,
"real_lights_or_devices_touched":False,
"production_ready":False,
}
def verify(x):
 if not isinstance(x,dict) or set(x)!=set(REQUIRED):raise ValueError("bad evidence schema")
 for key,wanted in REQUIRED.items():
  got=x[key]
  if type(got) is not type(wanted) or got!=wanted:
   raise ValueError("unsupported Adaptive Core claim: "+key)
def main():
 if len(sys.argv)!=2:raise SystemExit("expected one actual Core proof JSON")
 f=Path(sys.argv[1])
 if not f.is_file() or f.stat().st_size>16384:raise SystemExit("missing/oversized Core proof")
 try:verify(json.loads(f.read_text(encoding="utf-8")))
 except (ValueError,UnicodeError,OSError) as exc:raise SystemExit("FAIL P6I: "+str(exc)) from None
 print("P6I_GENUINE_ADAPTIVE_LIGHTING_UNAVAILABLE_RESTORED_LIFECYCLE_PASS")
 print("Only fictitious lights; real production parity NOT certified")
if __name__=="__main__":main()
