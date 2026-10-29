#!/usr/bin/env python3
"""Synthetic, local metadata-only backup inspector tests. No real HAOS files."""
from __future__ import annotations
from io import BytesIO
import json
from pathlib import Path
import tarfile
import tempfile
from inspect_ha_backup import inspect_backup, InvalidBackup

def archive(path:Path, entries:list[tuple[str,bytes,str]]) -> None:
    with tarfile.open(path,"w") as tar:
        for name,data,kind in entries:
            info=tarfile.TarInfo(name)
            info.size=len(data)
            if kind=="symlink":
                info.type=tarfile.SYMTYPE
                info.linkname="/etc/passwd"
                info.size=0
            elif kind=="hardlink":
                info.type=tarfile.LNKTYPE
                info.linkname="backup.json"
                info.size=0
            tar.addfile(info,BytesIO(data) if kind not in ("symlink", "hardlink") else None)

def manifest(protected=True, addons=None)->bytes:
    return json.dumps({
        "name":"synthetic private fixture",
        "slug":"mock-no-real-backup",
        "protected":protected,
        "homeassistant":{"version":"2026.10.1"},
        "addons":[{"slug":x} for x in (addons or ["core_matter_server"])],
        "compressed":True,
    }).encode()

def expect_error(path:Path,entries):
    archive(path,entries)
    try: inspect_backup(path)
    except InvalidBackup: return
    raise AssertionError("unsafe archive was accepted")

with tempfile.TemporaryDirectory(prefix="hars-backup-test-") as temp:
    path=Path(temp)/"synthetic.tar"
    archive(path,[
        ("backup.json",manifest(), "regular"),
        ("homeassistant.tar.gz",b"\x1f\x8bFAKE-GZIP-HEADER-NO-SECRETS","regular"),
        ("core_matter_server.tar.gz",b"SecureTarFAKE-SYNTHETIC-FILE","regular"),
    ])
    x=inspect_backup(path,{"core_matter_server","local_custom"})
    assert x["addons_advertised_count"]==1
    assert x["expected_installed_addons_count"]==2
    assert x["missing_installed_addons_count"]==1
    assert x["addon_payloads_identified_count"]==1
    assert x["homeassistant_payload_identified"] is True
    assert x["part_envelopes"]["securetar"]==1
    assert x["part_envelopes"]["gzip_plain"]==1
    assert x["can_restore"] is False
    assert "protected_flag_but_plain_gzip_detected" in x["warnings"]
    assert "missing_installed_addons_in_backup" in x["warnings"]
    print("PASS: synthetic HA backup manifest, missing App detection and mixed-envelope warning")
    expect_error(path,[("backup.json",manifest(),"regular"),("../escape",b"bad","regular")])
    expect_error(path,[("backup.json",manifest(),"regular"),("/absolute",b"bad","regular")])
    expect_error(path,[("backup.json",manifest(),"regular"),("escape",b"","symlink")])
    expect_error(path,[("backup.json",manifest(),"regular"),("backup.json",manifest(),"regular")])
    expect_error(path,[("backup.json",b"BAD-JSON","regular")])
    expect_error(path,[("backup.json",manifest(addons=["a","a"]),"regular")])
    expect_error(path,[("homeassistant.tar.gz",b"\x1f\x8b","regular")])
    expect_error(path,[("backup.json",manifest(),"regular"),("hardlink_entry",b"","hardlink")])
    expect_error(path,[("backup.json",manifest(),"regular"),("secrets.yaml",b"api_password: 123","regular")])
    expect_error(path,[("backup.json",manifest(),"regular"),(".env",b"SECRET_KEY=leak","regular")])
    expect_error(path,[("backup.json",json.dumps({"name":"leak","slug":"leak","password":"bad"}).encode(),"regular")])
    expect_error(path,[("backup.json",json.dumps({"name":"leak","slug":"leak","secret":"bad"}).encode(),"regular")])
    print("PASS: traversal/absolute/symlink/hardlink/secrets-exclusion/duplicates/invalid-metadata/missing-manifest rejected")
    archive(path,[("backup.json",manifest(protected=False),"regular"),
                  ("homeassistant.tar.gz",b"SecureTar-SYNTHETIC","regular")])
    report=inspect_backup(path)
    assert "unprotected_flag_but_securetar_detected" in report["warnings"]
    print("PASS: unprotected metadata with SecureTar-like envelope flagged")
    # Real Supervisor writes parts with a leading "./" prefix. This MUST be
    # accepted safely and normalised without allowing nested ".." components.
    archive(path,[
        ("./backup.json",json.dumps({"type":"full","protected":True,
            "addons":[],"homeassistant":{"version":"2026.10.1"}}).encode(),"regular"),
        ("./homeassistant.tar.gz",b"SecureTarFICTIONAL","regular"),
        ("./supervisor.tar.gz",b"SecureTarFICTIONAL","regular"),
    ])
    good=inspect_backup(path)
    assert good["supervisor_payload_identified"] is True,good
    assert good["supervisor_envelope"]=="securetar",good
    assert not good["blocking_issues"],good
    assert good["eligible_for_migration"] is False
    print("PASS: canonical ./ Supervisor tar accepted with correctly protected component")
    archive(path,[
        ("backup.json",json.dumps({"type":"full","protected":False,"addons":[]}).encode(),"regular"),
        ("homeassistant.tar.gz",b"\x1f\x8bFAKE","regular"),
        ("./supervisor.tar.gz",b"SecureTarFICTIONAL","regular"),
    ])
    mismatch=inspect_backup(path)
    assert "supervisor_encryption_mismatch" in mismatch["blocking_issues"],mismatch
    print("PASS: supervisor.tar encryption mismatch flags a blocking migration risk")
    archive(path,[
        ("./backup.json",manifest(),"regular"),
        ("backup.json",manifest(),"regular"),
    ])
    try: inspect_backup(path)
    except InvalidBackup: pass
    else: raise AssertionError("canonical duplicate accepted")
    print("PASS: duplicate ./backup.json vs backup.json rejected")
    expect_error(path,[("backup.json",manifest(),"regular"),("./sub/../escape",b"no","regular")])
    print("PASS: nested traversal still refused")
    # Duplicate JSON members are a metadata-smuggling vector even with one
    # canonical tar manifest. Never let json.loads silently take last value.
    for payload in (
        b'{"type":"partial","type":"full","addons":[]}',
        b'{"protected":false,"protected":true,"addons":[]}',
        b'{"addons":[{"slug":"a","slug":"b"}]}',
        b'{"addons":[],"homeassistant":{"version":"1","version":"2"}}',
        b'{"addons":[],"size":NaN}',
    ):
        expect_error(path,[("backup.json",payload,"regular")])
    print("PASS: duplicate nested JSON keys and nonfinite metadata rejected")

    # A preflight must open the inode itself rather than checking a pathname
    # then following a race-replaced symbolic link. Also refuse FIFOs without
    # waiting forever for a writer and without opening an actual HAOS file.
    live_target=Path(temp)/"synthetic-existing.tar"
    archive(live_target,[("backup.json",manifest(),"regular")])
    fake_link=Path(temp)/"trick-link.tar"
    fake_link.symlink_to(live_target)
    try: inspect_backup(fake_link)
    except InvalidBackup: pass
    else: raise AssertionError("symbolic link accepted as backup")
    fifo=Path(temp)/"trick-pipe.tar"
    import os
    os.mkfifo(fifo)
    try: inspect_backup(fifo)
    except InvalidBackup: pass
    else: raise AssertionError("FIFO was accepted as backup")
    print("PASS: nofollow inode-backed reader rejects symlink and FIFO without blocking")

print("PASS: fixture temporary archives removed")
