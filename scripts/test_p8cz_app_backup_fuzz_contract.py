#!/usr/bin/env python3
"""P8cz: reproducible fictional two-App Supervisor backup fuzz corpus.

No household archive, real credentials, devices, network or App activation.
"""
import argparse
import gzip
import hashlib
import io
import json
import subprocess
import tarfile
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "fuzz/corpus/app_backup_import"

def tar_bytes(members):
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w", format=tarfile.GNU_FORMAT) as tar:
        for name, data, kind in members:
            info = tarfile.TarInfo("./" + name)
            info.mode = 0o600
            info.mtime = 0
            if kind == "symlink":
                info.type = tarfile.SYMTYPE
                info.linkname = "/etc/passwd"
            else:
                info.size = len(data)
            tar.addfile(info, io.BytesIO(data) if kind == "file" else None)
    return buf.getvalue()

def inner(slug="core_configurator", *, symlink=False, traversal=False,
          duplicate=False, trailing=False):
    name = "../outside" if traversal else "data/" + slug + ".txt"
    first = (name, b"fictional P8cz App data only\n", "symlink" if symlink else "file")
    members = [first]
    if duplicate:
        members.append((name, b"duplicate fictional data", "file"))
    raw = tar_bytes(members)
    if trailing:
        raw += b"NONZERO AFTER TAR END"
    return gzip.compress(raw, mtime=0)

def outer(*, addons=("core_configurator",), payloads=None,
          protected=False, duplicate_outer=False, duplicate_json=False):
    if payloads is None:
        payloads = {"core_configurator": inner()}
    manifest = json.dumps(
        {"type": "partial", "protected": protected, "addons": list(addons)},
        separators=(",", ":"),
    ).encode()
    if duplicate_json:
        manifest = (
            b'{"type":"partial","protected":false,"protected":true,'
            b'"addons":["core_configurator"]}'
        )
    members = [("backup.json", manifest, "file")]
    members += [(slug + ".tar.gz", payload, "file")
                for slug, payload in payloads.items()]
    if duplicate_outer:
        members.append(("backup.json", b"{}", "file"))
    return tar_bytes(members)

def fixtures():
    first = inner()
    second = inner("lab_metrics")
    result = {
        "valid-one-fictional-app.tar": outer(),
        "valid-two-fictional-apps.tar": outer(
            addons=("core_configurator", "lab_metrics"),
            payloads={"core_configurator": first, "lab_metrics": second},
        ),
        "reject-protected.tar": outer(protected=True),
        "reject-duplicate-json-key.tar": outer(duplicate_json=True),
        "reject-duplicate-outer-member.tar": outer(duplicate_outer=True),
        "reject-inner-symlink.tar": outer(payloads={"core_configurator": inner(symlink=True)}),
        "reject-inner-traversal.tar": outer(payloads={"core_configurator": inner(traversal=True)}),
        "reject-inner-duplicate.tar": outer(payloads={"core_configurator": inner(duplicate=True)}),
        "reject-truncated-gzip.tar": outer(payloads={"core_configurator": first[:12]}),
        "reject-nonzero-tar-trailer.tar": outer(payloads={"core_configurator": inner(trailing=True)}),
        "reject-concatenated-gzip.tar": outer(
            payloads={"core_configurator": first + gzip.compress(b"NONZERO SECOND MEMBER", mtime=0)},
        ),
        "reject-undeclared-second-app.tar": outer(
            payloads={"core_configurator": first, "lab_metrics": second},
        ),
        "reject-outer-header-checksum.tar": None,
        "reject-nontar-bytes.tar": b"\x99" * 2048,
    }
    corrupt = bytearray(outer())
    corrupt[3] ^= 1
    result["reject-outer-header-checksum.tar"] = bytes(corrupt)
    return result

def assert_scope():
    fuzz = (ROOT / "fuzz/fuzz_targets/app_backup_import.rs").read_text()
    workflow = (ROOT / ".github/workflows/fuzz.yml").read_text()
    manifest = (ROOT / "fuzz/Cargo.toml").read_text()
    for needle in (
        "inspect_unprotected_app_only", "Sha256::digest(input)",
        "!proof.actions_executable && !proof.production_ready",
        "proof.app_count", "fs::read(&source)",
    ):
        assert needle in fuzz, needle
    for needle in (
        "name = \"app_backup_import\"",
        "path = \"fuzz_targets/app_backup_import.rs\"",
    ):
        assert needle in manifest, needle
    for needle in (
        "fuzz run app_backup_import -- -max_total_time=45",
        "fuzz run app_backup_import -- -max_total_time=180",
        "test_p8cz_app_backup_fuzz_contract.py",
        "crates/interop-moli/src/app_backup_import.rs",
    ):
        assert needle in workflow, needle
    for banned in (
        "pull_request_target:", "sudo podman", "pct exec",
        "qm stop 100", "192.168.", "GITHUB_TOKEN",
    ):
        assert banned not in fuzz + workflow, banned

def native_check(binary, cases):
    binary = binary.resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="p8cz-native-fuzz-") as tmp:
        private = Path(tmp) / "escrow"
        private.mkdir(mode=0o700)
        for name, raw in cases.items():
            source = Path(tmp) / name
            source.write_bytes(raw)
            source.chmod(0o600)
            run = subprocess.run(
                [str(binary), "--lab-readonly", str(private),
                 "app-source-ingest", str(source)],
                capture_output=True, text=True, timeout=35,
            )
            if name.startswith("valid-"):
                assert run.returncode == 0, (name, run.stderr)
                report = json.loads(run.stdout)
                expected_count = 2 if name.startswith("valid-two") else 1
                assert report["sha256"] == hashlib.sha256(raw).hexdigest(), name
                assert report["app_count"] == expected_count, name
                assert report["files"] == expected_count, name
                assert report["production_ready"] is False
                assert report["apps_restored"] is False
                assert report["actions_executable"] is False
            else:
                assert run.returncode != 0, (name, run.stdout)
                assert not run.stdout, name
        assert len(list(private.glob("*.tar"))) == 2
    print("P8CZ_NATIVE_TWO_FICTIONAL_APP_ACCEPTANCE_AND_TWELVE_ADVERSARIAL_REJECTIONS_PASS")

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--generate", action="store_true")
    parser.add_argument("--binary", type=Path)
    args = parser.parse_args()
    cases = fixtures()
    if args.generate:
        CORPUS.mkdir(parents=True, exist_ok=True)
        for name, data in cases.items():
            (CORPUS / name).write_bytes(data)
    assert CORPUS.is_dir()
    assert {p.name for p in CORPUS.iterdir()} == set(cases)
    for name, expected in cases.items():
        actual = (CORPUS / name).read_bytes()
        assert actual == expected, name
        assert 1024 <= len(actual) <= 2_097_152, name
    assert_scope()
    print("P8CZ_SYNTHETIC_APP_ONLY_CORPUS_AND_FUZZ_SCOPE_PASS")
    if args.binary:
        native_check(args.binary, cases)

if __name__ == "__main__":
    main()
