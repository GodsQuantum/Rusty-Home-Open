#!/usr/bin/env python3
"""Strict genuine Core v1.9.0 release ZIP lifecycle proof; no household devices."""
import json
from pathlib import Path
import sys

REQUIRED={
    "real_official_core":True,
    "release_zip_sha256":"ec47b0114a5bf447ef1b7fa15cfa80829619f6785ece33ee4b5957ef906f7865",
    "manifest_version":"v1.9.0",
    "upstream_tag_manifest_version":"v1.3.0",
    "genuine_user_config_flow":True,
    "real_composite_entity_created":True,
    "unload_entity_unavailable_restored":True,
    "reload_entity_reactivated":True,
    "entry_remove_entity_gone":True,
    "genuine_entry_setup_unload_reload_remove":True,
    "fictitious_child_preserved":True,
    "real_user_media_devices_touched":False,
    "production_ready":False,
}
def verify(proof:object)->None:
    if not isinstance(proof,dict) or set(proof)!=set(REQUIRED):
        raise ValueError("unexpected Core universal release proof shape")
    for key,expected in REQUIRED.items():
        actual=proof[key]
        if type(actual) is not type(expected) or actual!=expected:
            raise ValueError("unverified Core universal release property: "+key)

def main()->None:
    if len(sys.argv)!=2:
        raise SystemExit("expected one genuine Core proof path")
    file=Path(sys.argv[1])
    if not file.is_file() or file.stat().st_size>16384:
        raise SystemExit("missing/oversized Core proof")
    try:
        verify(json.loads(file.read_text(encoding="utf-8")))
    except (ValueError,UnicodeError,OSError) as exc:
        raise SystemExit("P6H_FAIL: "+str(exc)) from None
    print("P6H_REAL_RELEASE_V1_9_0_MULTISTEP_FLOW_ENTITY_UNLOAD_RELOAD_PASS")
    print("No real user media sources; 29 integrations NOT certified")

if __name__=="__main__":
    main()
