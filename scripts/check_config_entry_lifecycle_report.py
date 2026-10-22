#!/usr/bin/env python3
"""Strict proof of a real disposable Home Assistant Core config entry lifecycle.

This accepts ONLY a test-produced JSON file; it is NOT a 29-integration
certification, authenticated user proof or production-readiness attestation.
"""
import json
from pathlib import Path
import sys

REQUIRED = {
    "real_core_config_flow": True,
    "real_core_config_entry_created": True,
    "real_core_entry_loaded": True,
    "real_core_unload_state_removed": True,
    "real_core_reload_state_restored": True,
    "real_core_remove_hook": True,
    "offline_loopback_only": True,
    "synthetic_integration_count": 1,
    "actual_29_hacs_integrations_certified": 0,
    "duplicate_config_entry_denied": True,
    "production_ready": False,
}

def verify(payload: object) -> None:
    if not isinstance(payload, dict) or set(payload) != set(REQUIRED):
        raise ValueError("invalid Core lifecycle evidence shape")
    for key, expected in REQUIRED.items():
        found = payload[key]
        if type(found) is not type(expected) or found != expected:
            raise ValueError("Core lifecycle proof mismatch: " + key)

def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("expected actual disposable Core JSON artifact")
    path = Path(sys.argv[1])
    if not path.is_file() or path.stat().st_size > 16384:
        raise SystemExit("missing or oversized Core lifecycle report")
    try:
        verify(json.loads(path.read_text(encoding="utf-8")))
    except (ValueError, UnicodeError, OSError) as exc:
        raise SystemExit("P6C_REAL_CORE_LIFECYCLE_FAILED: " + str(exc)) from None
    print("P6C_GENUINE_HA_CORE_CONFIG_FLOW_SETUP_UNLOAD_RELOAD_REMOVE_PASS")
    print("1 fictional integration only; 0/29 real integrations certified")

if __name__ == "__main__":
    main()
