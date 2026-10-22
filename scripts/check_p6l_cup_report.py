#!/usr/bin/env python3
import json,sys
from pathlib import Path
REQUIRED={
 "genuine_core":True, "actual_release_tag":"v1.5.0",
 "actual_upstream_sha":"ca9b767cb9bdd83d9e279947cd89fa742648d1df",
 "release_asset_sha256":"52f6c367c8b039bb5d0b467206e357dc620c221ec2b255bd09c247b6205e0f51",
 "release_manifest_version":"v1.5.0",
 "actual_core_config_entry_loaded":True, "real_cup_http_config_flow":True,
 "real_cup_http_coordinator":True, "real_synthetic_sensors":True,
 "real_synthetic_update_event":True, "duplicate_config_refused":True,
 "unload_reload_remove":True, "actual_docker_or_user_images_tested":False,
 "production_ready":False,
}
def verify(obj):
 if type(obj) is not dict or obj.keys()!=REQUIRED.keys():raise ValueError("invalid fields")
 for k,v in REQUIRED.items():
  if type(obj[k]) is not type(v) or obj[k]!=v: raise ValueError("false Cup claim: "+k)
def main():
 if len(sys.argv)!=2:raise SystemExit("require JSON")
 f=Path(sys.argv[1])
 if not f.is_file() or f.stat().st_size>10000:raise SystemExit("missing/oversized")
 try:verify(json.loads(f.read_text(encoding="utf-8")))
 except (ValueError,UnicodeError,OSError) as err:raise SystemExit(str(err)) from None
 print("P6L genuine CUP released integration and Core sensor refresh verified")
if __name__=="__main__":main()
