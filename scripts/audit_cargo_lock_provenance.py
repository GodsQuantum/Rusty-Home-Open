#!/usr/bin/env python3
"""Read-only deterministic provenance verification of Cargo.lock.

This is NOT an SPDX/CycloneDX SBOM or license compliance review. Cargo.lock
does not contain complete crate licenses, NOTICE/assets or vulnerability data.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import re
import tomllib
from urllib.parse import urlsplit, parse_qs

HEX40 = re.compile(r"[0-9a-f]{40}\Z")
HEX64 = re.compile(r"[0-9a-f]{64}\Z")


def inventory(content: bytes) -> dict:
    lock = tomllib.loads(content.decode("utf-8"))
    packages = lock.get("package")
    if not isinstance(packages, list) or not packages:
        raise ValueError("missing Cargo.lock packages")
    rows, unique = [], set()
    for p in packages:
        if not isinstance(p, dict):
            raise ValueError("invalid package item")
        name, version, origin = p.get("name"), p.get("version"), p.get("source")
        if not all(isinstance(x, str) and x and len(x) <= 256 for x in (name, version)):
            raise ValueError("missing package identity")
        if origin is not None and (not isinstance(origin, str) or len(origin) > 2048):
            raise ValueError("invalid source")
        key = (name, version, origin)
        if key in unique:
            raise ValueError("duplicate locked package identity")
        unique.add(key)
        row = {"name": name, "version": version}
        if origin is None:
            row["source_type"] = "workspace"
        elif origin.startswith("registry+https://"):
            registry = urlsplit(origin[len("registry+"):])
            if registry.username or registry.password or registry.hostname is None:
                raise ValueError("registry source contains credentials or invalid host")
            checksum = p.get("checksum")
            if not isinstance(checksum, str) or not HEX64.fullmatch(checksum):
                raise ValueError("registry crate without locked SHA-256")
            row.update(source_type="registry", checksum_sha256=checksum)
            # Do not include alternate registry URLs with credentials in a report.
            row["registry"] = "https"
        elif origin.startswith("git+https://"):
            # Cargo.lock source form includes both '?rev=<SHA>' and '#<SHA>'.
            source = origin[4:]
            split = urlsplit(source)
            revision = parse_qs(split.query).get("rev", [])
            pinned = split.fragment
            if (
                split.username or split.password or split.scheme != "https"
                or split.hostname is None
                or len(revision) != 1
                or not HEX40.fullmatch(revision[0])
                or not HEX40.fullmatch(pinned)
                or revision[0] != pinned
            ):
                raise ValueError("Git dependency not fully pinned to one commit")
            row.update(source_type="git", git_sha=pinned, git_host=split.hostname)
        else:
            raise ValueError("unsupported or unreviewed Cargo source")
        rows.append(row)
    rows.sort(key=lambda x: (x["name"], x["version"], x["source_type"]))
    counts = {typ: sum(p["source_type"] == typ for p in rows)
              for typ in ("registry", "git", "workspace")}
    return {
        "format": "hars-cargo-provenance-1",
        "packages": rows,
        "counts": counts,
        "packages_total": len(rows),
        "checksum_git_pin_validation": "passed",
        "sbom_complete": False,
        "transitive_license_review_complete": False,
        "security_advisory_scan_complete": False,
        "release_gate_third_party_licenses_certified": False,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lock", type=Path, default=Path(__file__).resolve().parent.parent / "Cargo.lock")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    payload = json.dumps(inventory(args.lock.read_bytes()), indent=2, ensure_ascii=True) + "\n"
    if args.output:
        args.output.write_text(payload, encoding="utf-8")
    else:
        report = json.loads(payload)
        print(json.dumps({k: report[k] for k in
                          ("counts", "packages_total", "checksum_git_pin_validation",
                           "sbom_complete", "release_gate_third_party_licenses_certified")},
                         indent=2))


if __name__ == "__main__":
    main()
