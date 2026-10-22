#!/usr/bin/env python3
"""Fail-closed official Core Card Builder 2.6.0 synthetic lifecycle result."""
import json
from pathlib import Path
import sys

REQUIRED = {
    "real_source": "studiobts/home-assistant-card-builder",
    "upstream_tag": "v2.6.0",
    "upstream_commit": "2ae8e5152a5f6af46307130a045133f900f4397e",
    "zip_release_sha256": "75ca0cc33397b16f2d2af4c2aad40391a84f013786c264a22b7186fe309fb7a7",
    "core_config_flow_and_load": True,
    "core_frontend_asset_registered": True,
    "core_unload_and_reload": True,
    "core_remove": True,
    "real_user_dashboard_tested": False,
    "production_ready": False,
}

def verify(data):
    if not isinstance(data, dict) or set(data) != set(REQUIRED):
        raise ValueError("unexpected Card Builder synthetic HA Core evidence shape")
    for key, expected in REQUIRED.items():
        if type(data[key]) is not type(expected) or data[key] != expected:
            raise ValueError("Card Builder Core evidence mismatch: " + key)

def main():
    if len(sys.argv) != 2:
        raise SystemExit("expected genuine isolated Core JSON file path")
    p = Path(sys.argv[1])
    if not p.is_file() or p.stat().st_size > 16384:
        raise SystemExit("missing/oversized Core Card Builder proof")
    try:
        verify(json.loads(p.read_text(encoding="utf-8")))
    except (OSError, ValueError, UnicodeError) as exc:
        raise SystemExit("P6G_CARD_BUILDER_CORE_FAIL: " + str(exc)) from None
    print("P6G_GENUINE_CARD_BUILDER_2_6_CORE_CONFIG_FLOW_FRONTEND_LIFECYCLE_PASS")
    print("Synthetic-only native UI wiring; real users and dashboards untested.")

if __name__ == "__main__":
    main()
