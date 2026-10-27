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

def read_staged_supervisor_options(stage):
    # Pin the validated directory inode. Using a dirfd prevents directory
    # substitution between the metadata security check and the file open.
    stage=Path(stage)
    try:
        directory_fd=os.open(stage,os.O_RDONLY|os.O_DIRECTORY|
                             os.O_NOFOLLOW|os.O_NONBLOCK|os.O_CLOEXEC)
    except OSError as exc:
        raise SavedOptionsRefused("missing or unsafe App private directory") from exc
    try:
        directory_stat=os.fstat(directory_fd)
        if (not stat.S_ISDIR(directory_stat.st_mode) or
            directory_stat.st_uid!=os.geteuid() or
            directory_stat.st_mode&0o077):
            raise SavedOptionsRefused("App directory not owned private 0700")
        try:
            fd=os.open("addon.json",os.O_RDONLY|os.O_NOFOLLOW|
                       os.O_NONBLOCK|os.O_CLOEXEC,dir_fd=directory_fd)
        except OSError as exc:
            raise SavedOptionsRefused("missing or unsafe official addon.json") from exc
        try:
            file_stat=os.fstat(fd)
            if (not stat.S_ISREG(file_stat.st_mode) or file_stat.st_nlink!=1 or
                file_stat.st_uid!=os.geteuid() or file_stat.st_mode&0o077 or
                file_stat.st_size<20 or file_stat.st_size>MAX_ADDON_JSON):
                raise SavedOptionsRefused("unsafe official App metadata inode")
            with os.fdopen(fd,"rb",closefd=False) as stream:
                data=stream.read(MAX_ADDON_JSON+1)
            if len(data)!=file_stat.st_size:
                raise SavedOptionsRefused("metadata short or concurrent read")
            # Reject a concurrent write to this same open regular inode.
            after=os.fstat(fd)
            if (after.st_size!=file_stat.st_size or
                after.st_mtime_ns!=file_stat.st_mtime_ns or
                after.st_ctime_ns!=file_stat.st_ctime_ns):
                raise SavedOptionsRefused("metadata changed during read")
            try:
                metadata=json.loads(data,object_pairs_hook=_unique_pairs,
                    parse_constant=_nonfinite)
            except (ValueError,UnicodeDecodeError) as exc:
                raise SavedOptionsRefused("invalid official App metadata") from exc
        finally:
            os.close(fd)
    finally:
        os.close(directory_fd)
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
