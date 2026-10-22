#!/usr/bin/env python3
"""Certification reason changes cannot silently promote a lab to production."""
import json
from pathlib import Path
import unittest
from check_release_gate import audit
ROOT=Path(__file__).resolve().parent.parent

class ReleaseEvidenceTruth(unittest.TestCase):
    def setUp(self):
        self.data=json.loads((ROOT/"config/release-gates.json").read_text())
    def test_exact_fifteen_unapproved_and_unsigned(self):
        report=audit(self.data)
        self.assertEqual(report["total_gates"],15)
        self.assertEqual(report["blocking_gates"],15)
        self.assertFalse(report["deployment_authorized"])
        self.assertFalse(report["independent_release_signoff"])
        self.assertEqual(report["status"],"NOT_PRODUCTION_READY")
        for row in self.data["required_gates"]:
            self.assertIn(row["status"],("blocked","lab_partial"))
            self.assertIsNone(row["certification_artifact"])
            self.assertEqual(len(row["blocked_reason"]),1)
    def test_real_laboratory_evidence_not_described_as_absent(self):
        reasons={r["id"]:r["blocked_reason"][0].lower() for r in self.data["required_gates"]}
        for key,value in (
            ("haos_backup_restore","14/14"),
            ("ha_core_hacs_29_runtime","137/137"),
            ("ha_core_hacs_29_runtime","26 integrations"),
            ("packaging_updates","signed offline apt"),
            ("security_stress_fuzz","libfuzzer"),
            ("supervisor_apps_restore","nine actual"),
            ("apps_9_lifecycle","nine supervisor apps"),
            ("third_party_licenses","six cargo"),
        ):
            self.assertIn(value,reasons[key],key)
    def test_evidence_report_is_explicit_about_remaining_risks(self):
        text=(ROOT/"docs/CERTIFICATION_STATUS_2026-10-22.md").read_text()
        for required in (
            "NOT_PRODUCTION_READY","15 barrières","137/137",
            "29/29","14/14","synthetic VM","--assert-ready",
            "independent_release_signoff=false",
        ):
            self.assertIn(required,text)
if __name__=="__main__":
    unittest.main()
