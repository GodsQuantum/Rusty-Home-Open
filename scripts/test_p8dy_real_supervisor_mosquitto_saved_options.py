#!/usr/bin/env python3
"""Genuine HAOS18.3 official Mosquitto backup -> verified Rust private
data -> official persisted addon.json readiness; fail-closed without HA auth.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from inspect_ha_backup import inspect_backup
from supervisor_mosquitto_saved_options_lab import (
    SavedOptionsRefused,read_staged_supervisor_options,
    local_auth_migration_readiness
)
from haos_mosquitto_local_auth_lab import check_options, MigrationRefused

p=argparse.ArgumentParser()
p.add_argument("--archive",required=True,type=Path)
p.add_argument("--manifest",required=True,type=Path)
p.add_argument("--native",required=True,type=Path)
a=p.parse_args()
assert os.environ.get("GITHUB_ACTIONS")=="true" and os.environ.get("RUNNER_TEMP")
assert os.geteuid()!=0
archive=a.archive.resolve(strict=True)
receipt=json.loads(a.manifest.read_text())
with archive.open("rb") as f:
    digest=hashlib.file_digest(f,"sha256").hexdigest()
assert archive.stat().st_size==receipt["size_bytes"] and digest==receipt["sha256"]
assert inspect_backup(archive,{"core_mosquitto"})["app_only_partial_archive_identified"]
with tempfile.TemporaryDirectory(prefix="p8dy-real-haos-options-",dir=os.environ["RUNNER_TEMP"]) as t:
    root=Path(t)
    escrow=root/"escrow";escrow.mkdir(mode=0o700)
    output=root/"inactive";output.mkdir(mode=0o700)
    def stage(*params):
        x=subprocess.run([str(a.native),"--lab-readonly",str(escrow),*map(str,params)],
            capture_output=True,text=True,timeout=130)
        assert x.returncode==0 and "password" not in x.stdout.lower(), (
            "native Rust private App staging refused: "+x.stderr[-150:])
        data=json.loads(x.stdout)
        assert data["production_ready"] is False
        return data
    assert stage("app-source-ingest",archive)["sha256"]==digest
    assert stage("app-source-verify",digest)["archive_sha256"]==digest
    staged=stage("app-source-stage-inactive",digest,"core_mosquitto",output)
    assert staged["app_runtime_started"] is False
    private=output/("app-core_mosquitto-"+digest)
    assert private.is_dir()
    marker=list(private.rglob("rusty_home_p8do_nonsecret_mqtt_marker.txt"))
    assert len(marker)==1
    info=local_auth_migration_readiness(private)
    assert info["source"]=="actual_native_staged_official_supervisor_addon_json"
    assert info["production_ready"] is False
    # Official fresh HAOS default has NO fictitious local login; cannot
    # claim migration of Core/Supervisor delegated MQTT user authentication.
    actual=read_staged_supervisor_options(private)
    assert isinstance(actual,dict)
    assert not actual.get("logins"), (
        "Genuine disposable HAOS Mosquitto backup unexpectedly had local users")
    assert info["local_auth_eligible"] is False, (
        "Default HAOS auth delegation falsely claimed converted")
    try:
        check_options(actual)
    except MigrationRefused:
        pass
    else:
        raise AssertionError("Actual official Supervisor default user auth bypassed")
    assert all(key not in info for key in ("options","logins","password","token"))
    print("P8DY_ACTUAL_OFFICIAL_SUPERVISOR_BACKUP_ADDON_JSON_PERSISTED_OPTIONS_READ_PASS",flush=True)
    print("P8DY_REAL_HAOS_MOSQUITTO_EMPTY_LOCAL_LOGINS_DELEGATED_AUTH_FAILS_CLOSED_PASS",flush=True)
    print("P8DY_NATIVE_RUST_SHA_EXACT_PRIVATE_SOURCE_NO_SECRET_DISCLOSURE_PASS",flush=True)
    # Negative injected metadata cases are ALWAYS in separate synthetic
    # temporary dirs; never modify source archive or its native staging.
    for name,body in [
        ("duplicate",b'{"user":{"options":{},"options":{}},"system":{},"version":"1"}'),
        ("nonfinite",b'{"user":{"options":{"debug":NaN}},"system":{},"version":"1"}'),
        ("wrong_fields",b'{"user":{"other":{}},"system":{},"version":"1"}'),
        ("wrong_layout",b'{"user":[],"system":{},"version":"1"}'),
        ("truncated",b'{"user":'),
    ]:
        temp=root/("fictional_"+name);temp.mkdir(mode=0o700)
        f=temp/"addon.json";f.write_bytes(body);f.chmod(0o600)
        try:
            read_staged_supervisor_options(temp)
        except SavedOptionsRefused:
            pass
        else:
            raise AssertionError("Malicious "+name+" metadata unexpectedly accepted")
    link=root/"synthetic_link";link.mkdir(mode=0o700)
    (link/"addon.json").symlink_to(private/"addon.json")
    try:
        read_staged_supervisor_options(link)
    except SavedOptionsRefused:
        pass
    else:
        raise AssertionError("symlinked Supervisor metadata unexpectedly accepted")
    assert len(list(output.iterdir()))==1 and marker[0].is_file()
    with archive.open("rb") as h:
        assert hashlib.file_digest(h,"sha256").hexdigest()==digest
    print("P8DY_MALICIOUS_DUPLICATE_NONFINITE_WRONG_LAYOUT_SYMLINK_REFUSED_PASS",flush=True)
    print("P8DY_REAL_HAOS_ARCHIVE_UNTOUCHED_ZERO_PRODUCTION_ACTION_PASS",flush=True)
