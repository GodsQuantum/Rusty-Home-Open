#!/usr/bin/env python3
"""Ensure dated release roadmap accounts for every mandatory gate, never grants release."""
from pathlib import Path
import json
import re
import unittest
ROOT=Path(__file__).resolve().parent.parent

class CertificationCoverage(unittest.TestCase):
    def test_roadmap_exactly_covers_all_15_real_gates(self):
        config=json.loads((ROOT/"config/release-gates.json").read_text())
        rows=config["required_gates"]
        expected={x["id"] for x in rows}
        self.assertEqual(len(rows),15)
        self.assertEqual(len(expected),15)
        plan=(ROOT/"docs/PLAN_CERTIFICATION_TECHNOLOGIES_2026-10-23.md").read_text()
        found=re.findall(r"(?m)^\| ([a-z][a-z0-9_]+) \| (?:P0|P1|P2) \|",plan)
        self.assertEqual(len(found),len(set(found)),"duplicate gate rows")
        self.assertEqual(set(found),expected,"roadmap missing or inventing release gates")
        self.assertIn("15/15",plan)
        self.assertIn("Marketplace",plan)
        for x in rows:
            self.assertNotEqual(x["status"],"certified")
            self.assertIsNone(x["certification_artifact"])
    def test_no_release_authority_from_lab_migration_fixture(self):
        source=(ROOT/"crates/community-packages/src/marketplace_transition.rs").read_text()
        for text in ("activation_authorized: false","applies_changes: false","critical_hold"):
            self.assertIn(text,source)

if __name__=="__main__": unittest.main()
