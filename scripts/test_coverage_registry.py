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


if __name__ == "__main__":
    unittest.main()
