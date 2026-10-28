#!/usr/bin/env python3
"""P8ef: private synthetic Rust App pair must reject source/destination overlap.

No host mounts, network, real HAOS archives, devices, or production state.
"""
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile

binary = str(Path(sys.argv[1]).resolve(strict=True))

def archive(path, slug, payload):
    body = io.BytesIO()
    with tarfile.open(fileobj=body, mode="w:gz") as tf:
        info = tarfile.TarInfo("./data/p8ef-marker.txt")
        info.size = len(payload)
        tf.addfile(info, io.BytesIO(payload))
    with tarfile.open(path, mode="w") as tf:
        for name, data in (("./backup.json", json.dumps({"type": "partial", "protected": False,
                            "addons": [slug]}).encode()), (f"./{slug}.tar.gz", body.getvalue())):
            info = tarfile.TarInfo(name)
            info.size = len(data)
            tf.addfile(info, io.BytesIO(data))
    path.chmod(0o600)
    return hashlib.sha256(path.read_bytes()).hexdigest()

def call(escrow, *args):
    return subprocess.run([binary, "--lab-readonly", str(escrow), *map(str, args)],
                          capture_output=True, text=True, timeout=45)

with tempfile.TemporaryDirectory(prefix="p8ef-pair-overlap-") as scratch:
    root = Path(scratch)
    escrow = root/"escrow"; escrow.mkdir(mode=0o700)
    target = root/"target"; target.mkdir(mode=0o700)
    child = escrow/"child"; child.mkdir(mode=0o700)
    slugs = ("core_configurator", "core_mosquitto")
    source = [root/f"fixture{i}.tar" for i in range(2)]
    digests = [archive(path, slug, f"P8EF_FAKE_{i}\n".encode())
               for i, (path, slug) in enumerate(zip(source, slugs))]
    for path, digest in zip(source, digests):
        result = call(escrow, "app-source-ingest", path)
        assert result.returncode == 0, result.stderr
        assert json.loads(result.stdout)["sha256"] == digest
    originals = {digest: (escrow/f"{digest}.tar").read_bytes() for digest in digests}
    def stage(dest, pair_id):
        return call(escrow, "app-source-stage-pair-inactive", pair_id,
                    slugs[0], digests[0], slugs[1], digests[1], dest)
    for label, dest in (
        ("same-inode-dotdot", escrow/".."/"escrow"),
        ("descendant", child),
        ("ancestor", root),
    ):
        result = stage(dest, "p8ef-"+label)
        assert result.returncode != 0 and not result.stdout, (label, result.returncode, result.stdout)
        assert "HA_REGISTRY_STAGING_FAIL_CLOSED" in result.stderr, (label, result.stderr)
        assert not (dest/f"pair-p8ef-{label}").exists()
        assert all((escrow/f"{digest}.tar").read_bytes() == data for digest, data in originals.items())
        print(f"P8EF_PAIR_{label.upper().replace('-', '_')}_SOURCE_DESTINATION_OVERLAP_REFUSED_PASS")
    alias = root/"ancestor-link"
    alias.symlink_to(escrow, target_is_directory=True)
    result = stage(alias/"." , "p8ef-ancestor-symlink")
    assert result.returncode != 0 and not result.stdout
    assert not (escrow/"pair-p8ef-ancestor-symlink").exists()
    print("P8EF_PAIR_ANCESTOR_SYMLINK_ALIAS_REFUSED_PASS")
    result = stage(target, "p8ef-disjoint")
    assert result.returncode == 0, result.stderr
    receipt = json.loads(result.stdout)
    assert receipt["atomic_private_pair_published"] is True
    assert receipt["production_ready"] is False and receipt["app_runtime_started"] is False
    pair = target/"pair-p8ef-disjoint"
    assert pair.is_dir()
    for i, (slug, digest) in enumerate(zip(slugs, digests)):
        assert (pair/f"app-{slug}-{digest}"/"data/p8ef-marker.txt").read_bytes() == f"P8EF_FAKE_{i}\n".encode()
    assert all((escrow/f"{digest}.tar").read_bytes() == data for digest, data in originals.items())
    print("P8EF_DISJOINT_PAIR_REAL_NATIVE_STAGE_PRESERVES_SHA_EXACT_ESCROW_PASS")
