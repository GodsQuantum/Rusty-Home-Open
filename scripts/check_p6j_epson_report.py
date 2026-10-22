#!/usr/bin/env python3
"""Strict Core negative Epson 0.1.3 offline SNMP test: no real printer proof."""
import json
import sys
from pathlib import Path

REQUIRED={
 "genuine_core":True,
 "upstream_git_sha":"3a4f2c0120de7bfbf149041d484eefcc0b0cad0c",
 "upstream_version":"0.1.3",
 "pysnmp_version":"7.1.22",
 "native_core_config_flow_created":True,
 "offline_fictitious_host_not_loaded":True,
 "fake_live_sensors_absent":True,
 "duplicate_host_rejected":True,
 "entry_removed":True,
 "real_printer_touched":False,
 "positive_printer_lifecycle_certified":False,
 "production_ready":False,
}
def verify(x):
 if not isinstance(x,dict) or set(x)!=set(REQUIRED):
  raise ValueError("wrong Epson proof schema")
 for key,expected in REQUIRED.items():
  actual=x[key]
  if type(actual) is not type(expected) or actual!=expected:
   raise ValueError("Epson/Core false evidence: "+key)

def main():
 if len(sys.argv)!=2:raise SystemExit("need real Core proof file")
 f=Path(sys.argv[1])
 if not f.is_file() or f.stat().st_size>16384:raise SystemExit("missing/oversized proof")
 try:verify(json.loads(f.read_text(encoding="utf-8")))
 except (ValueError,UnicodeError,OSError) as e:
  raise SystemExit("P6J_FAIL: "+str(e)) from None
 print("P6J_GENUINE_EPSON_CONFIG_ENTRY_OFFLINE_DENIAL_PASS")
 print("No SNMP printer contacted; actual printer functionality NOT certified")
if __name__=="__main__":main()
