#!/usr/bin/env python3
"""Read-only private proof of TRUE official Supervisor App-only backup bytes.

Only the fixed nonsecret fixture may be inspected. No extraction to filesystem,
no native App activation, no decrypt, household credentials or production.
"""
import argparse,hashlib,io,json,re,tarfile
from pathlib import Path
from inspect_ha_backup import inspect_backup,normalized_archive_name,InvalidBackup

p=argparse.ArgumentParser()
p.add_argument("--archive",type=Path,required=True)
p.add_argument("--manifest",type=Path,required=True)
a=p.parse_args()
archive=a.archive.resolve(strict=True)
manifest=json.loads(a.manifest.read_text())
assert set(manifest)=={"slug","source_boot_id","core_version","sha256","size_bytes"}
assert re.fullmatch(r"[0-9a-f]{64}",manifest["sha256"])
assert re.fullmatch(r"[0-9a-f-]{36}",manifest["source_boot_id"])
assert 1024<manifest["size_bytes"]<1024*1024*1024
assert archive.stat().st_size==manifest["size_bytes"]
with archive.open("rb") as stream:
    assert hashlib.file_digest(stream,"sha256").hexdigest()==manifest["sha256"]
result=inspect_backup(archive,expected={"core_configurator"})
assert result["app_only_partial_archive_identified"] is True,result["blocking_issues"]
assert result["homeassistant_payload_identified"] is False
assert result["addons_advertised_count"]==1
assert result["addon_payloads_identified_count"]==1
assert result["missing_installed_addons_count"]==0
assert result["part_envelopes"]["securetar"]==0
assert result["part_envelopes"]["unrecognized"]==0
assert result["part_envelopes"]["gzip_plain"]==1
assert result["protected_metadata_flag"] is False
assert result["can_restore"] is False and result["eligible_for_migration"] is False
print("P8CU_GENUINE_OFFICIAL_HAOS_APP_ONLY_PARTIAL_PREFLIGHT_PASS")
marker=b"P8CU_OFFICIAL_SUPERVISOR_APP_ONLY_PRIVATE_MARKER_20261027\\n"
marker_basename="rusty_home_p8cu_nonsecret_synthetic_marker.txt"
found=0
# Read only the fixed app tar member and only a bounded marker; reject
# canonical path duplicates, links, oversized files and malformed members.
with tarfile.open(archive,mode="r:*") as outer:
    names={}
    for member in outer:
        if member.name in (".","./") and member.isdir():continue
        name=normalized_archive_name(member.name)
        assert name not in names,"duplicate Supervisor tar member"
        names[name]=member
    assert "homeassistant.tar.gz" not in names
    assert "homeassistant.tar" not in names
    part=names.get("core_configurator.tar.gz")
    assert part is not None and part.isfile()
    with outer.extractfile(part) as fileobj:
        with tarfile.open(fileobj=fileobj,mode="r|gz") as nested:
            seen=set()
            for index,entry in enumerate(nested):
                assert index<20000,"nested App archive exceeds limit"
                if entry.name in (".","./") and entry.isdir():continue
                name=normalized_archive_name(entry.name)
                assert name not in seen,"duplicate canonical nested App path"
                seen.add(name)
                assert entry.isfile() or entry.isdir(),"App links/devices denied"
                assert 0<=entry.size<128*1024*1024
                if name.endswith("/"+marker_basename) or name==marker_basename:
                    assert entry.isfile() and entry.size==len(marker),"synthetic marker size mismatch"
                    with nested.extractfile(entry) as stream:
                        content=stream.read(1024)
                    assert content==marker, "actual official backup lost or altered marker"
                    found+=1
assert found==1,"official backup omitted or duplicated the true fixture bytes"
print("P8CU_GENUINE_OFFICIAL_SUPERVISOR_ADDON_BYTES_EXACT_PRIVATE_MARKER_PASS")
print("P8CU_APP_ONLY_BACKUP_NATIVE_MIGRATION_AND_RUNTIME_NOT_AUTHORIZED_PASS")
