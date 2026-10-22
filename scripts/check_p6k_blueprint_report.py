#!/usr/bin/env python3
"""Exact structured positive and negative claims: reject fabricated prod status."""
import json
import sys
from pathlib import Path
REQUIRED = {
    "genuine_core": True, "upstream_source": "ha-china/blueprint-studio",
    "upstream_tag": "v2.5.2", "upstream_sha": "943aed0be67f3a910b0f70a3864288d5e17e55ce",
    "installed_manifest_version": "2.5.2", "paramiko_present": True,
    "actual_core_config_entry_loaded": True,
    "actual_core_http_anonymous_401": True,
    "actual_core_http_nonadmin_get_403": True,
    "actual_core_http_nonadmin_write_403": True,
    "actual_core_http_admin_read_200": True,
    "actual_core_http_admin_write_200": True,
    "admin_created_and_read_synthetic_file": True,
    "actual_core_demoted_admin_bearer_denied": True,
    "duplicate_config_refused": True,
    "unload_reload_remove": True, "real_user_data_tested": False,
    "production_ready": False,
}
def verify(data):
    if type(data) is not dict or data.keys()!=REQUIRED.keys():
        raise ValueError("wrong/missing extra Blueprint Studio fields")
    for key, value in REQUIRED.items():
        if type(data[key]) is not type(value) or data[key]!=value:
            raise ValueError("false or unproven Core claim: "+key)
def main():
    if len(sys.argv)!=2: raise SystemExit("Need one real Core JSON proof")
    f=Path(sys.argv[1])
    if not f.is_file() or f.stat().st_size > 12000:
        raise SystemExit("Missing/oversized proof")
    try: verify(json.loads(f.read_text(encoding="utf-8")))
    except (ValueError,UnicodeError,OSError) as exc:
        raise SystemExit("P6K_DENIED: "+str(exc)) from None
    print("P6K verified: genuine Blueprint Core API admin and nonadmin isolated lab")
if __name__=="__main__":main()
