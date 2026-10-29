#!/usr/bin/env python3
"""Read-only, metadata-only Home Assistant backup preflight. NEVER extracts.

No password handling: full SecureTar v3 restore must use official
Home Assistant/Supervisor or verified upstream securetar, not homegrown crypto.
All output is sanitized counts and known installed addon slugs; not secrets.
"""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path, PurePosixPath
import stat
import tarfile

MAX_MANIFEST = 1_048_576
MAX_MEMBERS = 25_000
MAX_ARCHIVE_SIZE = 1024**4
MAX_MEMBER_SIZE = 1024**4
MAX_HEADER = 32
BANNED_MANIFEST_KEYS = frozenset({"password", "api_password", "access_token", "refresh_token", "secret"})

class InvalidBackup(ValueError):
    pass

def parse_backup_manifest(raw: bytes) -> dict:
    """Fail closed on duplicate JSON keys (including nested addon metadata).

    Without pairs_hook, a malicious archive can present two different
    "protected", "type", or "addons" objects; the last silently wins.
    """
    def unique_pairs(pairs):
        obj = {}
        for name, value in pairs:
            if name in obj:
                raise InvalidBackup("duplicate metadata key")
            obj[name] = value
        return obj
    def reject_nonfinite(_):
        raise InvalidBackup("nonfinite backup metadata")
    try:
        metadata = json.loads(raw, object_pairs_hook=unique_pairs,
                              parse_constant=reject_nonfinite)
    except (ValueError, UnicodeDecodeError) as exc:
        raise InvalidBackup("invalid backup.json") from exc
    if not isinstance(metadata, dict):
        raise InvalidBackup("backup metadata must be object")
    if any(k in metadata for k in BANNED_MANIFEST_KEYS):
        raise InvalidBackup("prohibited secret key exposed in unencrypted manifest")
    return metadata

def normalized_archive_name(s: str) -> str:
    """Allow Supervisor's one leading './', reject all other dot traversal.

    Supervisor currently writes archives as './slug.tar.gz'. Normalize only
    the exact leading prefix; never call Path.resolve or follow a link.
    """
    if not s or "\\" in s or "\x00" in s or any(ord(c) < 32 for c in s):
        raise InvalidBackup("invalid archive path")
    if s.startswith("./"):
        s = s[2:]
    candidate = PurePosixPath(s)
    if (not s or candidate.is_absolute() or
        any(x in (".", "..", "") for x in s.rstrip("/").split("/"))):
        raise InvalidBackup("unsafe archive path")
    return s.rstrip("/")

def safe_archive_path(s: str) -> bool:
    try:
        normalized_archive_name(s)
        return True
    except InvalidBackup:
        return False

def get_expected(path: Path) -> set[str]:
    obj = json.loads(path.read_text(encoding="utf-8"))
    items = obj.get("apps", [])
    if not isinstance(items, list):
        raise InvalidBackup("invalid sanitized baseline")
    out = {x["slug"] for x in items if isinstance(x, dict) and isinstance(x.get("slug"), str)}
    return out

def inspect_backup(path: Path, expected: set[str] | None = None) -> dict:
    # Open the exact inode read-only with O_NOFOLLOW: preflight must not
    # check one path then accidentally consume a replaced symbolic link.
    # O_NONBLOCK avoids hangs on malicious FIFO paths before fstat validation.
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK)
    except OSError as exc:
        raise InvalidBackup("cannot safely open backup") from exc
    member_count = 0
    entries: dict[str, tarfile.TarInfo] = {}
    with os.fdopen(fd, "rb") as stream:
        info = os.fstat(stream.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_ARCHIVE_SIZE:
            raise InvalidBackup("backup must be a regular file within size limits")
        with tarfile.open(fileobj=stream, mode="r:*") as tf:
            # Only read tar headers; never unpack nested secrets/config.
            for member in tf:
                member_count += 1
                if member_count > MAX_MEMBERS:
                    raise InvalidBackup("too many archive members")
                if member.name in (".", "./") and member.isdir():
                    continue  # benign tar root-directory marker
                name = normalized_archive_name(member.name)
                if name in entries:
                    raise InvalidBackup("duplicate canonical tar member name")
                if member.issym() or member.islnk() or member.isdev() or member.isfifo():
                    raise InvalidBackup("link/device entries prohibited")
                if not (member.isfile() or member.isdir()):
                    raise InvalidBackup("unsupported archive entry type")
                if member.size < 0 or member.size > MAX_MEMBER_SIZE:
                    raise InvalidBackup("archive member exceeds size limit")
                if not member.isdir() and name != "backup.json" and not (name.endswith(".tar.gz") or name.endswith(".tar")):
                    raise InvalidBackup("loose non-archive file prohibited in backup root")
                entries[name] = member
            manifest = entries.get("backup.json")
            if manifest is None or not manifest.isfile() or manifest.size > MAX_MANIFEST:
                raise InvalidBackup("missing/oversized backup.json")
            fp = tf.extractfile(manifest)
            if fp is None:
                raise InvalidBackup("cannot read backup.json")
            with fp:
                raw = fp.read(MAX_MANIFEST + 1)
            if len(raw) > MAX_MANIFEST:
                raise InvalidBackup("manifest too large")
            meta = parse_backup_manifest(raw)
            advertised_addons = meta.get("addons", [])
            if not isinstance(advertised_addons, list):
                raise InvalidBackup("metadata addons must be list")
            addons = set()
            for x in advertised_addons:
                name = x.get("slug") if isinstance(x, dict) else (x if isinstance(x, str) else None)
                if not isinstance(name, str) or not name or "/" in name or len(name) > 128:
                    raise InvalidBackup("invalid addon slug")
                if name in addons:
                    raise InvalidBackup("duplicate addon slug")
                addons.add(name)
            backup_type = meta.get("type")
            if backup_type is not None and backup_type not in ("full", "partial"):
                raise InvalidBackup("unexpected backup type")
            protected = meta.get("protected")
            if protected is not None and not isinstance(protected, bool):
                raise InvalidBackup("invalid protected flag")
            inner = sorted(n for n in entries if n.endswith((".tar.gz", ".tar")) and entries[n].isfile())
            formats: dict[str, int] = {"securetar": 0, "gzip_plain": 0, "unrecognized": 0}
            part_types: dict[str, str] = {}
            for n in inner:
                handle = tf.extractfile(entries[n])
                if handle is None:
                    raise InvalidBackup("unreadable archive part")
                with handle:
                    head = handle.read(MAX_HEADER)
                if head.startswith(b"SecureTar"):
                    kind = "securetar"
                elif head.startswith(b"\x1f\x8b"):
                    kind = "gzip_plain"
                else:
                    kind = "unrecognized"
                formats[kind] += 1
                part_types[n] = kind
            file_names = set(inner)
            # Match documented conventional Supervisor addon filenames, not
            # every possible custom addon packaging or encryption generation.
            candidate_addon_parts = {s for s in addons if s + ".tar.gz" in file_names or s + ".tar" in file_names}
            home_part = "homeassistant.tar.gz" in file_names or "homeassistant.tar" in file_names
            warnings = []
            blockers = []
            supervisor_part = next((n for n in ("supervisor.tar.gz", "supervisor.tar") if n in part_types), None)
            supervisor_kind = part_types.get(supervisor_part) if supervisor_part else None
            if supervisor_kind == "unrecognized":
                blockers.append("supervisor_payload_unrecognized")
            elif supervisor_part and protected is not None:
                expected_kind = "securetar" if protected else "gzip_plain"
                if supervisor_kind != expected_kind:
                    blockers.append("supervisor_encryption_mismatch")
            if backup_type == "full" and not supervisor_part:
                warnings.append("full_backup_without_supervisor_payload_check_upstream_version")
            if protected is True and formats["gzip_plain"] > 0:
                warnings.append("protected_flag_but_plain_gzip_detected")
                blockers.append("part_envelope_inconsistent_with_protection")
            if protected is False and formats["securetar"] > 0:
                warnings.append("unprotected_flag_but_securetar_detected")
                blockers.append("part_envelope_inconsistent_with_protection")
            if addons - candidate_addon_parts:
                warnings.append("declared_addon_payload_not_identified")
                blockers.append("advertised_addon_payload_missing")
            if expected and not expected.issubset(addons):
                warnings.append("missing_installed_addons_in_backup")
                blockers.append("expected_apps_not_in_backup")
            # Official Supervisor also writes PARTIAL backups containing only
            # one or more App payloads, without a homeassistant.tar.gz.
            # Previously those genuine archives were misclassified as broken.
            # Accept only a self-consistent *unprotected* app-only PARTIAL
            # envelope. This is structure preflight, never App restoration.
            app_only = backup_type == "partial" and not home_part and bool(addons)
            if not home_part:
                warnings.append("no_homeassistant_payload_identified")
                if not app_only:
                    blockers.append("homeassistant_payload_missing")
                elif protected is not False:
                    blockers.append("app_only_partial_requires_explicit_unprotected")
                elif candidate_addon_parts != addons:
                    blockers.append("app_only_partial_requires_all_addon_payloads")
            if app_only and formats["unrecognized"]:
                blockers.append("app_only_partial_envelope_unrecognized")
            core = meta.get("homeassistant", {})
            version = core.get("version") if isinstance(core, dict) else None
            # Version is optional and sanitized; don't print raw metadata.
            version = version if isinstance(version, str) and len(version) < 60 and all(ch.isascii() and (ch.isalnum() or ch in ".-_") for ch in version) else None
            return {
                "mode": "offline_metadata_only_no_decryption_no_extraction",
                "source": "synthetic_or_user_supplied_archive",
                "archive_member_count": member_count,
                "addons_advertised_count": len(addons),
                "addon_payloads_identified_count": len(candidate_addon_parts),
                "expected_installed_addons_count": len(expected) if expected else None,
                "expected_installed_addons_present_count": len(addons & expected) if expected else None,
                "missing_installed_addons_count": len(expected - addons) if expected else None,
                "homeassistant_payload_identified": home_part,
                "app_only_partial_archive_identified": app_only
                    and not blockers,
                "ha_core_version": version,
                "protected_metadata_flag": protected,
                "part_envelopes": formats,
                "supervisor_payload_identified": supervisor_part is not None,
                "supervisor_envelope": supervisor_kind,
                "blocking_issues": sorted(set(blockers)),
                "eligible_for_migration": False,
                "warnings": sorted(set(warnings)),
                "can_restore": False,
                "reason": "metadata preflight is not encrypted backup validation or restore parity"
            }

def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("backup", type=Path, help="Offline backup archive path (NEVER write to it)")
    ap.add_argument("--sanitized-baseline", type=Path, help="Known installed Apps inventory (sanitized JSON)")
    args = ap.parse_args()
    expected = get_expected(args.sanitized_baseline) if args.sanitized_baseline else None
    report = inspect_backup(args.backup, expected)
    print(json.dumps(report, sort_keys=True, indent=2))

if __name__ == "__main__":
    main()
