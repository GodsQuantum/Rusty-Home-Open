#!/usr/bin/env python3
"""Verify a genuine Watchman 0.8.4 report on exact Core 2026.10.4.

This verifies lab evidence only; never grants production deployment.
"""
from pathlib import Path
import json
import sys

from check_p6d_real_watchman_report import EXPECTED as BASE

EXPECTED={**BASE,"core_version":"2026.10.4"}

def verify(data):
    if not isinstance(data,dict) or set(data)!=set(EXPECTED):
        raise ValueError("missing, extra or altered version-specific report fields")
    for key,expected in EXPECTED.items():
        actual=data[key]
        if type(actual) is not type(expected) or actual!=expected:
            raise ValueError("Watchman 2026.10.4 Core version or lifecycle mismatch: "+key)

def main():
    if len(sys.argv)!=2:
        raise SystemExit("requires isolated current Core proof JSON")
    p=Path(sys.argv[1])
    if not p.is_file() or p.stat().st_size>16384:
        raise SystemExit("missing or oversized Core proof")
    try:
        verify(json.loads(p.read_text(encoding="utf-8")))
    except (ValueError, UnicodeError, OSError) as e:
        raise SystemExit("P8EC_CURRENT_CORE_HACS_NOT_VERIFIED: "+str(e)) from None
    print("P8EC_REAL_WATCHMAN_0_8_4_CORE_2026_10_4_LIFECYCLE_PASS")
    print("Only a new real upstream integration and Core matrix cell, production 0/15.")

if __name__ == "__main__":
    main()
