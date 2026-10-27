#!/usr/bin/env python3
"""P8dq: REAL native Rust two-App atomic private staging, faults and SIGKILL.

Entirely fictional TAR/GZIP data; CT700 or disposable CI only. No HAOS,
network, OCI runtime, actual user archive, radio, secret or host mount.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import signal
import subprocess
import tarfile
import tempfile
import time

parser = argparse.ArgumentParser()
parser.add_argument("--binary", required=True, type=Path)
args = parser.parse_args()
binary = str(args.binary.resolve(strict=True))
slugs = ("core_configurator", "core_mosquitto")
payloads = (b"P8DQ_FIRST_APP_FAKE_DATA\n", b"P8DQ_SECOND_APP_FAKE_DATA\n")

def archive(path, slug, payload):
    inner = io.BytesIO()
    with tarfile.open(fileobj=inner, mode="w:gz") as tf:
        info = tarfile.TarInfo("./data/marker.txt")
        info.size = len(payload)
        tf.addfile(info, io.BytesIO(payload))
    outer = {"type": "partial", "protected": False, "addons": [slug]}
    with tarfile.open(path, mode="w") as tf:
        for name, data in (("./backup.json", json.dumps(outer).encode()),
                           ("./" + slug + ".tar.gz", inner.getvalue())):
            info = tarfile.TarInfo(name)
            info.size = len(data)
            tf.addfile(info, io.BytesIO(data))
    path.chmod(0o600)
    return hashlib.sha256(path.read_bytes()).hexdigest()

def cmd(escrow, *items, env=None, success=True):
    run = subprocess.run([binary, "--lab-readonly", str(escrow), *map(str, items)],
                         text=True, capture_output=True, timeout=60, env=env)
    if not success:
        assert run.returncode != 0 and not run.stdout, (items, run)
        assert "HA_REGISTRY_STAGING_FAIL_CLOSED" in run.stderr
        return None
    assert run.returncode == 0, (items, run.stderr)
    result = json.loads(run.stdout)
    assert result["production_ready"] is False
    assert all(p.decode().strip() not in run.stdout for p in payloads)
    return result

def pair(escrow, target, pair_id, digests, **opts):
    return cmd(escrow, "app-source-stage-pair-inactive", pair_id,
               slugs[0], digests[0], slugs[1], digests[1], target, **opts)

with tempfile.TemporaryDirectory(prefix="p8dq-native-pair-") as scratch:
    root = Path(scratch)
    escrow = root / "escrow"
    target = root / "target"
    escrow.mkdir(mode=0o700)
    target.mkdir(mode=0o700)
    sources = [root / f"source-{i}.tar" for i in range(2)]
    digests = tuple(archive(p, s, v) for p, s, v in zip(sources, slugs, payloads))
    for path, digest in zip(sources, digests):
        assert cmd(escrow, "app-source-ingest", path)["sha256"] == digest
    assert len(list(escrow.glob("*.tar"))) == 2

    fail = dict(os.environ, HARS_P8DQ_FAIL_AFTER_FIRST="1")
    pair(escrow, target, "fault-one", digests, env=fail, success=False)
    assert not list(target.iterdir())
    fail = dict(os.environ, HARS_P8DQ_FAIL_AFTER_SECOND="1")
    pair(escrow, target, "fault-two", digests, env=fail, success=False)
    assert not list(target.iterdir())
    pair(escrow, target, "missing-second", (digests[0], "a"*64), success=False)
    assert not list(target.iterdir())
    print("P8DQ_NATIVE_TWO_APP_FIRST_SECOND_AND_MISSING_SOURCE_ATOMIC_ROLLBACK_PASS", flush=True)

    # Forged orphan with same prefix MUST NOT be removed.
    forged = target / ".inactive-pair-forged"
    forged.mkdir(mode=0o700)
    (forged / ".hars-pair-lease-v1").write_bytes(b"FORGED")
    forged.joinpath("preserve").write_bytes(b"not-ours")
    # Kill the actual Rust process after the FIRST App is privately staged.
    pause = dict(os.environ, HARS_P8DQ_PAUSE_AFTER_FIRST_MS="20000")
    process = subprocess.Popen(
        [binary, "--lab-readonly", str(escrow), "app-source-stage-pair-inactive",
         "kill-then-retry", slugs[0], digests[0], slugs[1], digests[1], str(target)],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=pause, text=True,
    )
    try:
        deadline = time.monotonic() + 14
        found = False
        while time.monotonic() < deadline:
            candidates = [p for p in target.iterdir()
                          if p.name.startswith(".inactive-pair-") and p != forged]
            if candidates and any((p / ("app-" + slugs[0] + "-" + digests[0])).is_dir()
                                  for p in candidates):
                found = True
                break
            if process.poll() is not None:
                break
            time.sleep(0.08)
        assert found and process.poll() is None, "real first App stage not observed before kill"
        process.send_signal(signal.SIGKILL)
        process.communicate(timeout=10)
        assert process.returncode == -signal.SIGKILL
        assert not (target / "pair-kill-then-retry").exists()
        assert forged.joinpath("preserve").read_bytes() == b"not-ours"
        assert any(p.name.startswith(".inactive-pair-") and p != forged
                   for p in target.iterdir()), "expected interrupted private orphan"
    finally:
        if process.poll() is None:
            process.kill()
            process.communicate(timeout=10)
    receipt = pair(escrow, target, "kill-then-retry", digests)
    assert receipt["app_count"] == 2 and receipt["atomic_private_pair_published"] is True
    assert receipt["app_runtime_started"] is False and receipt["live_data_modified"] is False
    published = target / "pair-kill-then-retry"
    assert published.is_dir() and published.stat().st_mode & 0o077 == 0
    for slug, digest, value in zip(slugs, digests, payloads):
        stage = published / ("app-" + slug + "-" + digest)
        assert stage.is_dir()
        marker = stage / "data" / "marker.txt"
        assert marker.read_bytes() == value and marker.stat().st_mode & 0o077 == 0
    assert not (published / ".hars-pair-lease-v1").exists()
    assert sorted(p.name for p in target.iterdir()) == [forged.name, published.name]
    assert forged.joinpath("preserve").read_bytes() == b"not-ours"
    print("P8DQ_REAL_NATIVE_SIGKILL_FIRST_APP_NO_PARTIAL_PUBLICATION_SAFE_REAP_RETRY_PASS", flush=True)

    pair(escrow, target, "kill-then-retry", digests, success=False)
    pair(escrow, target, "duplicate", (digests[0], digests[0]), success=False)
    cmd(escrow, "app-source-stage-pair-inactive", "duplicate-slug",
        slugs[0], digests[0], slugs[0], digests[1], target, success=False)
    source2 = escrow / (digests[1] + ".tar")
    raw = source2.read_bytes()
    source2.write_bytes(raw[:-1024] + b"X" + raw[-1023:])
    pair(escrow, target, "tampered-second", digests, success=False)
    assert not (target / "pair-tampered-second").exists()
    for slug, digest, value in zip(slugs, digests, payloads):
        assert (published / ("app-" + slug + "-" + digest) / "data/marker.txt").read_bytes() == value
    assert hashlib.sha256(sources[0].read_bytes()).hexdigest() == digests[0]
    assert hashlib.sha256(sources[1].read_bytes()).hexdigest() == digests[1]
    print("P8DQ_TAMPER_COLLISION_DUPLICATE_REFUSAL_EXISTING_PAIR_UNCHANGED_PASS", flush=True)
