#!/usr/bin/env python3
"""Adversarial and contract tests for the coverage registry and validator."""
import copy
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
import sys
sys.path.insert(0, str(ROOT / "scripts"))
from check_coverage_registry import validate_registry, validate_receipt


class TestCoverageRegistry(unittest.TestCase):
    def setUp(self):
        reg_file = ROOT / "config/coverage-registry.json"
        self.data = json.loads(reg_file.read_text(encoding="utf-8"))

    def test_canonical_passes(self):
        validate_registry(self.data)

    def test_proven_promotion_strictly_rejected(self):
        mutated = copy.deepcopy(self.data)
        mutated["apps"][0]["status"] = "proven"
        mutated["apps_summary"]["proven"] = 1
        mutated["apps_summary"]["lab_partial"] = 1
        with self.assertRaises(ValueError) as ctx:
            validate_registry(mutated)
        self.assertIn("falsely claimed 'proven'", str(ctx.exception))

    def test_production_status_tamper_rejected(self):
        mutated = copy.deepcopy(self.data)
        mutated["production_status"] = "PRODUCTION_CERTIFIED"
        with self.assertRaises(ValueError) as ctx:
            validate_registry(mutated)
        self.assertIn("production_status must remain NOT_PRODUCTION_READY", str(ctx.exception))

    def test_duplicate_app_slug_rejected(self):
        mutated = copy.deepcopy(self.data)
        mutated["apps"].append(copy.deepcopy(mutated["apps"][0]))
        mutated["apps_summary"]["registered"] = len(mutated["apps"])
        mutated["apps_summary"]["lab_partial"] += 1
        with self.assertRaises(ValueError) as ctx:
            validate_registry(mutated)
        self.assertIn("Duplicate app slug", str(ctx.exception))

    def test_duplicate_integration_domain_rejected(self):
        mutated = copy.deepcopy(self.data)
        mutated["integrations"].append(copy.deepcopy(mutated["integrations"][0]))
        mutated["integrations_summary"]["registered"] = len(mutated["integrations"])
        mutated["integrations_summary"]["lab_partial"] += 1
        with self.assertRaises(ValueError) as ctx:
            validate_registry(mutated)
        self.assertIn("Duplicate integration domain", str(ctx.exception))

    def test_mismatched_run_url_rejected(self):
        mutated = copy.deepcopy(self.data)
        mutated["apps"][0]["receipt"]["run_url"] = "https://github.com/spoofed/repo/actions/runs/123"
        with self.assertRaises(ValueError) as ctx:
            validate_registry(mutated)
        self.assertIn("run_url mismatch", str(ctx.exception))

    def test_forged_sha_rejected(self):
        mutated = copy.deepcopy(self.data)
        mutated["integrations"][0]["receipt"]["head_sha"] = "not_a_valid_sha"
        with self.assertRaises(ValueError) as ctx:
            validate_registry(mutated)
        self.assertIn("head_sha must be 40-char git commit SHA", str(ctx.exception))

    def test_summary_count_mismatch_rejected(self):
        mutated = copy.deepcopy(self.data)
        mutated["apps_summary"]["lab_partial"] += 1
        with self.assertRaises(ValueError) as ctx:
            validate_registry(mutated)
        self.assertIn("count mismatch", str(ctx.exception))

    def test_blocked_requires_absent_reason(self):
        mutated = copy.deepcopy(self.data)
        mutated["apps"][0]["status"] = "blocked"
        mutated["apps_summary"]["lab_partial"] = 1
        mutated["apps_summary"]["blocked"] = 1
        mutated["apps"][0]["receipt"] = {"present": False}
        with self.assertRaises(ValueError) as ctx:
            validate_registry(mutated)
        self.assertIn("blocked requires absent_reason", str(ctx.exception))

        # With valid absent_reason it passes receipt validation
        mutated["apps"][0]["receipt"]["absent_reason"] = "Requires physical Zigbee coordinator radio."
        validate_registry(mutated)

    def test_wave2_integrations_present_and_valid(self):
        wave2_expected = {
            "card_builder": "P6G_REAL_HACS_CARD_BUILDER_RELEASE_CORE_LIFECYCLE_PASS",
            "smart_announcements": "P6F_REAL_SMART_ANNOUNCEMENTS_PINNED_CORE_PASS",
            "adaptive_lighting": "P6F_REAL_ADAPTIVE_LIGHTING_V1_32_0_CORE_LIFECYCLE_PASS",
            "custom_universal_media_player": "P6H_REAL_PUBLIC_UNIVERSAL_MEDIA_PLAYER_1_9_0_LIFECYCLE_PASS",
            "epson_snmp": "P6J_GENUINE_EPSON_CONFIG_ENTRY_OFFLINE_DENIAL_PASS",
        }
        integrations_by_domain = {item["domain"]: item for item in self.data["integrations"]}
        for domain, expected_marker in wave2_expected.items():
            self.assertIn(domain, integrations_by_domain, f"Wave 2 domain {domain} missing")
            item = integrations_by_domain[domain]
            self.assertEqual(item["status"], "lab_partial")
            self.assertEqual(item["category"], "hacs_integration")
            receipt = item["receipt"]
            self.assertTrue(receipt["present"])
            self.assertEqual(receipt["run_id"], 40004128595)
            self.assertEqual(receipt["head_sha"], "7234873317765b4dc2ea87c82611b4dd061ba14a")
            self.assertEqual(receipt["marker"], expected_marker)


if __name__ == "__main__":
    unittest.main()
