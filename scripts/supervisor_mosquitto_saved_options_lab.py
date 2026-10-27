#!/usr/bin/env python3
"""Read-only, conservative LAB view of actual native-staged Supervisor addon.json.

The original archive must FIRST be imported and SHA-verified by Rust
ha_registry_stager. This reader never decrypts, changes, logs, redacts or
serializes account passwords and NEVER activates broker/anonymous auth.
Any ambiguous or unsupported layout is a hard denial.
"""
import json
import os
import stat
from pathlib import Path
from haos_mosquitto_local_auth_lab import check_options, MigrationRefused

class SavedOptionsRefused(ValueError):
    pass

MAX_ADDON_JSON=131072

def _unique_pairs(pairs):
    out={}
    for key,value in pairs:
        if key in out:
            raise SavedOptionsRefused("duplicate protected metadata key")
        out[key]=value
    return out

def _nonfinite(_):
    raise SavedOptionsRefused("non-finite option number")

def _owned_private(directory):
    if directory.is_symlink():
        raise SavedOptionsRefused("App private directory symlink")
    try:
        x=directory.lstat()
    except OSError as exc:
        raise SavedOptionsRefused("missing App private directory") from exc
    if not stat.S_ISDIR(x.st_mode) or x.st_uid!=os.geteuid() or x.st_mode&0o077:
        raise SavedOptionsRefused("App directory not owned private 0700")

def read_staged_supervisor_options(stage):
    stage=Path(stage)
    _owned_private(stage)
    source=stage/"addon.json"
    try:
        # Rust stager denies all tar member symlinks/hardlinks/devices;
        # repeat the O_NOFOLLOW, hardlink and size invariants independently.
        fd=os.open(source,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK|os.O_CLOEXEC)
    except OSError as exc:
        raise SavedOptionsRefused("missing or unsafe official addon.json") from exc
    try:
        st=os.fstat(fd)
        if (not stat.S_ISREG(st.st_mode) or st.st_nlink!=1 or
            st.st_uid!=os.geteuid() or st.st_mode&0o077 or
            st.st_size<20 or st.st_size>MAX_ADDON_JSON):
            raise SavedOptionsRefused("unsafe official App metadata inode")
        with os.fdopen(fd,"rb",closefd=False) as stream:
            data=stream.read(MAX_ADDON_JSON+1)
        if len(data)!=st.st_size:
            raise SavedOptionsRefused("metadata short or concurrent read")
        try:
            metadata=json.loads(data,object_pairs_hook=_unique_pairs,
                parse_constant=_nonfinite)
        except (ValueError,UnicodeDecodeError) as exc:
            raise SavedOptionsRefused("invalid official App metadata") from exc
    finally:
        os.close(fd)
    if not isinstance(metadata,dict) or not isinstance(metadata.get("user"),dict):
        raise SavedOptionsRefused("Supervisor user metadata absent or wrong type")
    if not isinstance(metadata.get("system"),dict):
        raise SavedOptionsRefused("Supervisor system metadata absent or wrong type")
    if not isinstance(metadata.get("version"),str):
        raise SavedOptionsRefused("Supervisor version metadata missing")
    user=metadata["user"]
    options=user.get("options")
    if not isinstance(options,dict):
        raise SavedOptionsRefused("Supervisor persisted options absent")
    return options

def local_auth_migration_readiness(stage):
    # Return only public counts/booleans. Credentials MUST NOT be logged.
    options=read_staged_supervisor_options(stage)
    try:
        check_options(options)
    except MigrationRefused:
        return {"source":"actual_native_staged_official_supervisor_addon_json",
                "local_auth_eligible":False,
                "production_ready":False,
                "reason":"unsupported_or_delegated_auth_or_tls_requires_manual_review"}
    return {"source":"actual_native_staged_official_supervisor_addon_json",
            "local_auth_eligible":True,
            "production_ready":False,
            "reason":"eligible_only_for_disposable_local_credentials_lab"}
