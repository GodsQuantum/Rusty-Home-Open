#!/usr/bin/env python3
"""Strict Core lab proof for genuine Watchman 0.8.4 from pinned upstream."""
import json
from pathlib import Path
import sys

EXPECTED={
    "real_upstream":"dummylabs/thewatchman",
    "upstream_commit":"2eab21b7aeb4d4e20c71c3e2b3e449c42c4a3b35",
    "integration_version":"0.8.4",
    "dependency_prettytable":"3.12.0",
    "genuine_core_config_flow":True,
    "genuine_core_setup":True,
    "genuine_core_report_action":True,
    "genuine_core_report_executed_without_file":True,
    "genuine_core_unload_and_reload":True,
    "genuine_core_remove":True,
    "real_user_config_or_devices_tested":False,
    "production_ready":False,
}
def verify(x):
    if not isinstance(x,dict) or set(x)!=set(EXPECTED):
        raise ValueError("unexpected real Watchman proof shape")
    for key,expected in EXPECTED.items():
        actual=x[key]
        if type(actual) is not type(expected) or actual!=expected:
            raise ValueError("real Watchman evidence mismatch: "+key)
def main():
    if len(sys.argv)!=2:
        raise SystemExit("requires actual pinned Core proof JSON path")
    path=Path(sys.argv[1])
    if not path.is_file() or path.stat().st_size>16384:
        raise SystemExit("missing/oversized Watchman proof")
    try:
        verify(json.loads(path.read_text(encoding="utf-8")))
    except (ValueError,UnicodeError,OSError) as e:
        raise SystemExit("P6D_WATCHMAN_NOT_VERIFIED: "+str(e)) from None
    print("P6D_REAL_PINNED_WATCHMAN_CORE_SETUP_UNLOAD_RELOAD_REMOVE_PASS")
    print("One real public Watchman version, simulated config; production NOT certified.")
if __name__=="__main__":
    main()
