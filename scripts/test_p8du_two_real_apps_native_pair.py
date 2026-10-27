#!/usr/bin/env python3
"""P8du: two REAL official HAOS selected-App-only archives -> native Rust atomic pair.

Sources come from the SAME disposable HAOS boot with both Apps installed.
Real native SIGKILL, tampered second escrow refusal, exact byte preservation.
NO App activation, live data, home service, radio or user credentials.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time
from inspect_ha_backup import inspect_backup
from p8du_two_real_apps_qemu_pytest import APP_MARKERS

parser = argparse.ArgumentParser()
parser.add_argument("--editor", required=True, type=Path)
parser.add_argument("--editor-manifest", required=True, type=Path)
parser.add_argument("--mosquitto", required=True, type=Path)
parser.add_argument("--mosquitto-manifest", required=True, type=Path)
parser.add_argument("--native", required=True, type=Path)
a = parser.parse_args()
assert os.environ.get("GITHUB_ACTIONS") == "true" and os.environ.get("RUNNER_TEMP")
assert os.geteuid() != 0
native = str(a.native.resolve(strict=True))
slugs = ("core_configurator", "core_mosquitto")
sources = (a.editor.resolve(strict=True), a.mosquitto.resolve(strict=True))
manifests = (json.loads(a.editor_manifest.read_text()), json.loads(a.mosquitto_manifest.read_text()))
digests = []
for slug, source, meta in zip(slugs, sources, manifests):
    assert 1024 < source.stat().st_size == meta["size_bytes"] <= 1024*1024*1024
    with source.open("rb") as handle:
        sha = hashlib.file_digest(handle, "sha256").hexdigest()
    assert sha == meta["sha256"]
    inspected = inspect_backup(source, {slug})
    assert inspected["app_only_partial_archive_identified"] is True
    assert inspected["blocking_issues"] == []
    assert inspected["addons_advertised_count"] == 1
    assert inspected["addon_payloads_identified_count"] == 1
    assert inspected["can_restore"] is False
    digests.append(sha)
assert digests[0] != digests[1]
assert manifests[0]["source_boot_id"] == manifests[1]["source_boot_id"]
assert manifests[0]["core_version"] == manifests[1]["core_version"]
print("P8DU_TWO_GENUINE_OFFICIAL_APP_BACKUPS_SAME_GUEST_SHA_VERIFIED_PASS", flush=True)

def run(cmd, env=None, positive=True, timeout=120):
    p = subprocess.run(cmd, capture_output=True, text=True, env=env, timeout=timeout)
    if positive:
        assert p.returncode == 0, (p.returncode, p.stderr[-600:])
    else:
        assert p.returncode != 0 and not p.stdout
        assert "HA_REGISTRY_STAGING_FAIL_CLOSED" in p.stderr
    return p

with tempfile.TemporaryDirectory(prefix="p8du-real-haos-pair-", dir=os.environ["RUNNER_TEMP"]) as scratch:
    root = Path(scratch)
    escrow, target = root/"escrow", root/"inactive"
    escrow.mkdir(mode=0o700)
    target.mkdir(mode=0o700)
    def cmd(*args, env=None, positive=True):
        p = run([native, "--lab-readonly", str(escrow), *map(str, args)],
                env=env, positive=positive)
        if not positive:
            return None
        value = json.loads(p.stdout)
        assert value["production_ready"] is False
        assert all(marker[1].decode().strip() not in p.stdout for marker in APP_MARKERS.values())
        return value
    for source, digest in zip(sources, digests):
        assert cmd("app-source-ingest", source)["sha256"] == digest
        assert cmd("app-source-verify", digest)["archive_sha256"] == digest
    assert len(list(escrow.glob("*.tar"))) == 2

    # A genuinely intact first official archive must NOT publish when the
    # second genuine official escrow is corrupted, even if its metadata is valid.
    corrupted = escrow/(digests[1]+".tar")
    with corrupted.open("r+b") as handle:
        handle.seek(-1024, os.SEEK_END)
        byte = handle.read(1)
        assert len(byte) == 1
        handle.seek(-1024, os.SEEK_END)
        handle.write(bytes([byte[0] ^ 0x01]))
        handle.flush()
        os.fsync(handle.fileno())
    cmd("app-source-stage-pair-inactive", "real-corrupt-second",
        slugs[0], digests[0], slugs[1], digests[1], target, positive=False)
    assert not list(target.iterdir())
    # Restore only the disposable private escrow, never touch original archive.
    with sources[1].open("rb") as src, corrupted.open("wb") as dst:
        while True:
            chunk = src.read(1024*1024)
            if not chunk:
                break
            dst.write(chunk)
        dst.flush()
        os.fsync(dst.fileno())
    corrupted.chmod(0o600)
    assert cmd("app-source-verify", digests[1])["archive_sha256"] == digests[1]
    print("P8DU_GENUINE_SECOND_APP_CORRUPT_ESCROW_FAILS_CLOSED_NO_PAIR_PASS", flush=True)

    # Actual SIGKILL after FIRST genuine App staged but BEFORE atomic publish.
    pause = dict(os.environ, HARS_P8DQ_PAUSE_AFTER_FIRST_MS="20000")
    proc = subprocess.Popen(
        [native, "--lab-readonly", str(escrow), "app-source-stage-pair-inactive",
         "real-kill-retry", slugs[0], digests[0], slugs[1], digests[1], str(target)],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=pause)
    try:
        found = False
        deadline = time.monotonic() + 14
        while time.monotonic() < deadline:
            dirs = [p for p in target.iterdir() if p.name.startswith(".inactive-pair-")]
            if any((p/("app-"+slugs[0]+"-"+digests[0])).is_dir() for p in dirs):
                found = True
                break
            if proc.poll() is not None:
                break
            time.sleep(0.1)
        assert found and proc.poll() is None, "Real first official App not staged before SIGKILL"
        proc.send_signal(signal.SIGKILL)
        proc.communicate(timeout=10)
        assert proc.returncode == -signal.SIGKILL
        assert not (target/"pair-real-kill-retry").exists()
        assert any(p.name.startswith(".inactive-pair-") for p in target.iterdir())
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.communicate(timeout=10)
    print("P8DU_REAL_OFFICIAL_FIRST_APP_SIGKILL_ZERO_ATOMIC_PUBLICATION_PASS", flush=True)

    receipt = cmd("app-source-stage-pair-inactive", "real-kill-retry",
                  slugs[0], digests[0], slugs[1], digests[1], target)
    assert receipt["app_count"] == 2 and receipt["atomic_private_pair_published"] is True
    assert receipt["app_runtime_started"] is False and receipt["live_data_modified"] is False
    pair = target/"pair-real-kill-retry"
    assert pair.is_dir() and pair.stat().st_mode & 0o077 == 0
    for slug, digest in zip(slugs, digests):
        stage = pair/("app-"+slug+"-"+digest)
        filename, expected, _port = APP_MARKERS[slug]
        matches = list(stage.rglob(filename))
        assert len(matches) == 1 and matches[0].read_bytes() == expected
        assert stage.stat().st_mode & 0o077 == 0
        assert all(not p.is_symlink() and p.stat().st_mode & 0o077 == 0
                   for p in stage.rglob("*"))
    assert list(target.iterdir()) == [pair]
    cmd("app-source-stage-pair-inactive", "real-kill-retry",
        slugs[0], digests[0], slugs[1], digests[1], target, positive=False)
    for source, digest in zip(sources, digests):
        with source.open("rb") as handle:
            assert hashlib.file_digest(handle, "sha256").hexdigest() == digest
    print("P8DU_TWO_REAL_HAOS_APP_BYTES_ATOMIC_PRIVATE_RETRY_SOURCE_UNCHANGED_PASS", flush=True)

