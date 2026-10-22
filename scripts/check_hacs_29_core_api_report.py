#!/usr/bin/env python3
"""Strict official Core module-path audit: NOT real integration setup."""
import json
import sys
from pathlib import Path

REQUIRED = {
    "scope": "static_python_module_presence_not_runtime_compatibility",
    "core_version": "2026.10.1",
    "integrations_inventory": 29,
    "distinct_ha_import_paths": 137,
    "paths_resolvable": 137,
    "paths_unresolved": 0,
    "integrations_with_unresolved_import_paths": 0,
    "all_hacs_compatible": False,
    "config_flow_tested_for_29": False,
    "real_devices_touched": False,
    "unresolved_paths": [],
}

def validate(data):
    if not isinstance(data, dict) or set(data) != set(REQUIRED):
        raise ValueError("unexpected report shape")
    for key, value in REQUIRED.items():
        observed = data[key]
        if type(observed) is not type(value) or observed != value:
            raise ValueError("pinned official Core module audit mismatch: " + key)

def main():
    if len(sys.argv) != 2:
        raise SystemExit("one report JSON path required")
    p = Path(sys.argv[1])
    if not p.is_file() or p.stat().st_size > 32768:
        raise SystemExit("missing or oversized Core report")
    try:
        validate(json.loads(p.read_text(encoding="utf-8")))
    except (ValueError, UnicodeError, OSError) as exc:
        raise SystemExit("FAIL P6b: " + str(exc)) from None
    print("P6B_PINNED_CORE_29_INVENTORY_137_PATHS_PASS")
    print("NOT real integration setup; 0/29 functional certification.")

if __name__ == "__main__":
    main()
