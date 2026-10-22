#!/usr/bin/env python3
"""Fail-closed bounded discovery of a QEMU-only local Supervisor App version.

A timed-out POST /store/reload may have completed server-side. Re-read
`ha apps info` before repeating it. No production or general-purpose client.
"""
import re
from time import sleep as _sleep

_SLUG = re.compile(r"local_[a-z0-9_]{1,63}\Z")
_VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+\Z")

def ensure_supervisor_local_app_version(shell_json, slug, expected_version,
                                         *, sleeper=_sleep, max_reloads=2):
    if not isinstance(slug, str) or not _SLUG.fullmatch(slug):
        raise ValueError("invalid disposable local App identifier")
    if not isinstance(expected_version,str) or not _VERSION.fullmatch(expected_version):
        raise ValueError("invalid expected App version")
    if max_reloads not in (1,2):
        raise ValueError("unexpected reload budget")

    def observed():
        try:
            response = shell_json("ha apps info "+slug+" --no-progress --raw-json", timeout=45)
        except Exception:
            return None
        if not isinstance(response,dict) or response.get("result")!="ok":
            return None
        info=response.get("data")
        if not isinstance(info,dict) or info.get("slug")!=slug:
            return None
        if info.get("version_latest")==expected_version:
            return info
        return None

    for attempt in range(max_reloads):
        snapshot=observed()
        if snapshot is not None:
            return snapshot
        # The reload itself is a Supervisor-owned catalog refresh. The
        # next GET, not the POST return value, is the success condition.
        try:
            shell_json("ha store reload --no-progress --raw-json",timeout=120)
        except Exception:
            pass
        for poll in range(3):
            snapshot=observed()
            if snapshot is not None:
                return snapshot
            if poll<2:
                sleeper(2)
    raise RuntimeError(
        "Disposable Supervisor App catalogue failed expected version verification "
        "after bounded reloads (no activation attempted)"
    )
