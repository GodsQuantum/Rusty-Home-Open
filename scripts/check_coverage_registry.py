#!/usr/bin/env python3
"""Validator for Rusty Home 9-App and 29-Integration Coverage Registry.

Strict fail-closed schema, receipt, and provenance checker:
- Validates structure against schema requirements.
- Confirms counts match summary fields.
- Rejects unproven 'proven' status (must have full multi-host independent certification).
- Verifies exact receipt hashes, run URLs, and positive integers.
- Confirms production_status remains 'NOT_PRODUCTION_READY'.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
SLUG_RE = re.compile(r"^[a-z0-9_]+$")
REPO = "GodsQuantum/Rusty-Home-Open"
ALLOWED_STATUSES = {"proven", "lab_partial", "blocked"}
ALLOWED_APP_CATEGORIES = {"official_core_app", "community_app", "third_party_app"}
ALLOWED_INTEGRATION_CATEGORIES = {"core_integration", "hacs_integration"}


def validate_registry(data: dict) -> None:
    if not isinstance(data, dict):
        raise ValueError("Registry root must be a JSON object")

    if data.get("schema_version") != 1:
        raise ValueError("Unsupported schema_version; expected 1")

    if data.get("production_status") != "NOT_PRODUCTION_READY":
        raise ValueError("production_status must remain NOT_PRODUCTION_READY")

    apps_summary = data.get("apps_summary")
    if not isinstance(apps_summary, dict):
        raise ValueError("Missing or invalid apps_summary")
    if apps_summary.get("total_target") != 9:
        raise ValueError("apps_summary.total_target must be 9")

    integrations_summary = data.get("integrations_summary")
    if not isinstance(integrations_summary, dict):
        raise ValueError("Missing or invalid integrations_summary")
    if integrations_summary.get("total_target") != 29:
        raise ValueError("integrations_summary.total_target must be 29")

    apps = data.get("apps")
    if not isinstance(apps, list) or len(apps) < 2 or len(apps) > 9:
        raise ValueError("apps must be an array of length between 2 and 9")

    integrations = data.get("integrations")
    if not isinstance(integrations, list) or len(integrations) < 3 or len(integrations) > 29:
        raise ValueError("integrations must be an array of length between 3 and 29")

    # Validate apps
    seen_slugs = set()
    app_status_counts = {"proven": 0, "lab_partial": 0, "blocked": 0}
    for idx, app in enumerate(apps):
        if not isinstance(app, dict):
            raise ValueError(f"App at index {idx} must be an object")
        slug = app.get("slug")
        if not isinstance(slug, str) or not SLUG_RE.match(slug):
            raise ValueError(f"App {slug!r} invalid slug")
        if slug in seen_slugs:
            raise ValueError(f"Duplicate app slug: {slug}")
        seen_slugs.add(slug)

        category = app.get("category")
        if category not in ALLOWED_APP_CATEGORIES:
            raise ValueError(f"App {slug} invalid category: {category}")

        status = app.get("status")
        if status not in ALLOWED_STATUSES:
            raise ValueError(f"App {slug} invalid status: {status}")
        app_status_counts[status] += 1

        # Until independent production certification signoff, 'proven' is forbidden
        if status == "proven":
            raise ValueError(f"App {slug} falsely claimed 'proven'; requires independent signoff")

        notes = app.get("scope_notes")
        if not isinstance(notes, str) or len(notes) < 10:
            raise ValueError(f"App {slug} missing or short scope_notes")

        receipt = app.get("receipt")
        validate_receipt(f"App {slug}", status, receipt)

    # Validate integrations
    seen_domains = set()
    integration_status_counts = {"proven": 0, "lab_partial": 0, "blocked": 0}
    for idx, item in enumerate(integrations):
        if not isinstance(item, dict):
            raise ValueError(f"Integration at index {idx} must be an object")
        domain = item.get("domain")
        if not isinstance(domain, str) or not SLUG_RE.match(domain):
            raise ValueError(f"Integration {domain!r} invalid domain")
        if domain in seen_domains:
            raise ValueError(f"Duplicate integration domain: {domain}")
        seen_domains.add(domain)

        category = item.get("category")
        if category not in ALLOWED_INTEGRATION_CATEGORIES:
            raise ValueError(f"Integration {domain} invalid category: {category}")

        status = item.get("status")
        if status not in ALLOWED_STATUSES:
            raise ValueError(f"Integration {domain} invalid status: {status}")
        integration_status_counts[status] += 1

        if status == "proven":
            raise ValueError(f"Integration {domain} falsely claimed 'proven'; requires independent signoff")

        notes = item.get("scope_notes")
        if not isinstance(notes, str) or len(notes) < 10:
            raise ValueError(f"Integration {domain} missing or short scope_notes")

        receipt = item.get("receipt")
        validate_receipt(f"Integration {domain}", status, receipt)

    # Check summary counts match
    if apps_summary.get("registered") != len(apps):
        raise ValueError("apps_summary.registered does not match apps length")
    for st, count in app_status_counts.items():
        if apps_summary.get(st) != count:
            raise ValueError(f"apps_summary.{st} count mismatch: expected {count}, got {apps_summary.get(st)}")

    if integrations_summary.get("registered") != len(integrations):
        raise ValueError("integrations_summary.registered does not match integrations length")
    for st, count in integration_status_counts.items():
        if integrations_summary.get(st) != count:
            raise ValueError(f"integrations_summary.{st} count mismatch: expected {count}, got {integrations_summary.get(st)}")


def validate_receipt(entity_name: str, status: str, receipt: dict) -> None:
    if not isinstance(receipt, dict):
        raise ValueError(f"{entity_name} missing receipt object")

    present = receipt.get("present")
    if not isinstance(present, bool):
        raise ValueError(f"{entity_name} receipt.present must be boolean")

    if status == "blocked":
        if present:
            raise ValueError(f"{entity_name} is blocked but receipt.present is True")
        absent_reason = receipt.get("absent_reason")
        if not isinstance(absent_reason, str) or len(absent_reason) < 10:
            raise ValueError(f"{entity_name} blocked requires absent_reason >= 10 chars")
    elif status == "lab_partial":
        if not present:
            raise ValueError(f"{entity_name} is lab_partial but receipt.present is False")
        run_id = receipt.get("run_id")
        if not isinstance(run_id, int) or run_id <= 0:
            raise ValueError(f"{entity_name} receipt.run_id must be positive int")
        head_sha = receipt.get("head_sha")
        if not isinstance(head_sha, str) or not SHA_RE.match(head_sha):
            raise ValueError(f"{entity_name} receipt.head_sha must be 40-char git commit SHA")
        run_url = receipt.get("run_url")
        expected_url = f"https://github.com/{REPO}/actions/runs/{run_id}"
        if run_url != expected_url:
            raise ValueError(f"{entity_name} run_url mismatch: expected {expected_url}, got {run_url}")
        marker = receipt.get("marker")
        if not isinstance(marker, str) or len(marker) < 5:
            raise ValueError(f"{entity_name} missing or short receipt.marker")


def main() -> None:
    reg_path = ROOT / "config/coverage-registry.json"
    if len(sys.argv) > 1:
        reg_path = Path(sys.argv[1])
    if not reg_path.is_file():
        raise SystemExit(f"Coverage registry file not found: {reg_path}")
    data = json.loads(reg_path.read_text(encoding="utf-8"))
    try:
        validate_registry(data)
    except ValueError as exc:
        raise SystemExit(f"FAIL Coverage Registry Validation: {exc}")
    print(f"PASS: Coverage registry validated ({len(data['apps'])} apps, {len(data['integrations'])} integrations).")
    print(f"Status: {data['production_status']} (0/15 certified release gates).")


if __name__ == "__main__":
    main()
