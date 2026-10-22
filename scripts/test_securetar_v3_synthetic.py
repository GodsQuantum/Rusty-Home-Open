"""Official SecureTar v3 cryptographic round-trip on synthetic throwaway data.

Runs inside pinned Home Assistant Core Docker image without network or devices.
Never accepts a user backup path, password or real private configuration.
"""
from __future__ import annotations
from io import BytesIO
import json
import importlib.metadata
from pathlib import Path
import tarfile
from tempfile import TemporaryDirectory
from securetar import SecureTarArchive, SecureTarFile
from securetar import InvalidPasswordError
from inspect_ha_backup import inspect_backup

VERSION=importlib.metadata.version("securetar")
FAKE_PASSWORD="FAKE-SECURETAR-TEST-ONLY-NOT-AN-ACTUAL-SECRET"
NAME="homeassistant.tar.gz"
SYNTHETIC_CONFIG=b"homeassistant:\n  name: Synthetic Bridge Fixture\n"
with TemporaryDirectory(prefix="hars-securetar-v3-") as temp:
    path=Path(temp)/"synthetic-securetar-v3.tar"
    with SecureTarArchive(path,"w",password=FAKE_PASSWORD) as archive:
        with archive.create_tar(NAME,gzip=True) as inner:
            item=tarfile.TarInfo("data/configuration.yaml")
            item.size=len(SYNTHETIC_CONFIG)
            item.mode=0o600
            inner.addfile(item,BytesIO(SYNTHETIC_CONFIG))
        meta=json.dumps({"slug":"synthetic-only","compressed":True,
            "protected":True,"homeassistant":{"version":"2026.10.1"},
            "addons":[]}).encode()
        info=tarfile.TarInfo("backup.json")
        info.size=len(meta)
        archive.tar.addfile(info,BytesIO(meta))
    overview=inspect_backup(path,{"core_matter_server"})
    assert overview["part_envelopes"]["securetar"]==1,overview
    assert overview["protected_metadata_flag"] is True
    assert overview["homeassistant_payload_identified"] is True
    assert overview["can_restore"] is False
    print("PASS: HARS manifest inspector identifies genuine upstream encrypted SecureTar header")
    with SecureTarArchive(path,"r",password=FAKE_PASSWORD) as archive:
        names=[x.name for x in archive.tar]
        assert "backup.json" in names and NAME in names,names
        member=archive.tar.getmember(NAME)
        assert member.size>100
        assert archive.validate_password(member) is True
        print("PASS: official SecureTar v3 synthetic password header authentication")
    with SecureTarArchive(path,"r",password=FAKE_PASSWORD) as archive:
        member=archive.tar.getmember(NAME)
        assert archive.validate(member) is True
        print("PASS: official SecureTar v3 full encrypted content integrity")
    # Actual synthetic config-file restoration through upstream SecureTarFile.
    # NO user backup and NO output outside this disposable tmpfs/scratch path.
    restored=Path(temp)/"restored-synthetic-config"
    restored.mkdir()
    with SecureTarArchive(path,"r",password=FAKE_PASSWORD) as archive:
        member=archive.tar.getmember(NAME)
        with SecureTarFile(
            fileobj=archive.tar.extractfile(member),
            password=FAKE_PASSWORD,
        ) as inner:
            inner.extractall(path=restored,filter="data")
    recovered=restored/"data/configuration.yaml"
    assert recovered.read_bytes()==SYNTHETIC_CONFIG
    print("PASS: official SecureTar encrypted synthetic configuration restored byte-for-byte")
    hostile=Path(temp)/"hostile-securetar.tar"
    with SecureTarArchive(hostile,"w",password=FAKE_PASSWORD) as archive:
        with archive.create_tar(NAME,gzip=True) as inner:
            traversal=tarfile.TarInfo("../escape")
            traversal.size=len(SYNTHETIC_CONFIG)
            inner.addfile(traversal,BytesIO(SYNTHETIC_CONFIG))
    safe_dir=Path(temp)/"safe-import"
    safe_dir.mkdir()
    with SecureTarArchive(hostile,"r",password=FAKE_PASSWORD) as archive:
        with SecureTarFile(
            fileobj=archive.tar.extractfile(archive.tar.getmember(NAME)),
            password=FAKE_PASSWORD,
        ) as inner:
            try:
                inner.extractall(path=safe_dir,filter="data")
            except tarfile.FilterError:
                pass
            else:
                raise AssertionError("encrypted traversal accepted by upstream safe tar filter")
    assert not (Path(temp)/"escape").exists()
    print("PASS: malicious encrypted inner-tar path traversal blocked by tar filter")


    with SecureTarArchive(path,"r",password="INCORRECT-SYNTHETIC-TEST") as archive:
        member=archive.tar.getmember(NAME)
        try:
            result = archive.validate_password(member)
        except InvalidPasswordError:
            result = False
        assert result is False, "wrong password accepted"
        print("PASS: wrong synthetic password rejected")
    tampered=Path(temp)/"tampered.tar"
    tampered.write_bytes(path.read_bytes())
    with tarfile.open(tampered,"r") as tf:
        item=tf.getmember(NAME)
        position=item.offset_data+item.size-48
        assert item.offset_data+64 < position < item.offset_data+item.size
    with tampered.open("r+b") as f:
        f.seek(position)
        before=f.read(1)
        f.seek(position)
        f.write(bytes([before[0]^0x01]))
    with SecureTarArchive(tampered,"r",password=FAKE_PASSWORD) as archive:
        member=archive.tar.getmember(NAME)
        try:
            result=archive.validate(member)
        except Exception as exc:
            result=False
        assert result is False, "tampered encrypted backup accepted"
        print("PASS: tampered v3 encrypted stream rejected")
print("PASS: synthetic SecureTar v3 archive and fake password destroyed")
print("RESULT: official SecureTar",VERSION,"v3 TESTED; not HAOS backup restoration")
