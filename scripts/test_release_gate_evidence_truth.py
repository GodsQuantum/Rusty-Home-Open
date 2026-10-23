#!/usr/bin/env python3
"""Regression: every product gate has explicit scope, independent lab evidence and remaining risk."""
from pathlib import Path
import json
import unittest
from check_release_gate import audit
from verify_lab_evidence import validate
ROOT=Path(__file__).resolve().parent.parent

class GateReality(unittest.TestCase):
    def setUp(self):
        self.manifest=json.loads((ROOT/"config/release-gates.json").read_text())
        self.rows=self.manifest["required_gates"]
        self.claims=validate(json.loads((ROOT/"config/lab-evidence.json").read_text()))
    def test_all_fifteen_still_block_unapproved_production(self):
        state=audit(self.manifest)
        self.assertEqual(state["total_gates"],15)
        self.assertEqual(state["blocking_gates"],15)
        self.assertEqual(state["progress"],{"blocked":6,"lab_partial":9,"certified":0})
        self.assertFalse(state["deployment_authorized"])
        self.assertFalse(state["independent_release_signoff"])
        self.assertEqual(sum(x["status"]=="lab_partial" for x in self.rows),9)
        self.assertEqual(sum(x["status"]=="blocked" for x in self.rows),6)
        self.assertTrue(all(x["certification_artifact"] is None for x in self.rows))
    def test_real_evidence_never_waives_production_acceptance(self):
        self.assertEqual(len(self.claims),13)
        known={x["id"]:x for x in self.rows}
        self.assertEqual(set(known),{x["id"] for x in self.rows})
        self.assertTrue(all(x["blocked_reason"] and len(x["blocked_reason"][0])>80
                            for x in self.rows))
        self.assertTrue(all("Synthetic public alpha:" not in x["blocked_reason"][0]
                            for x in self.rows))
        for gate in {"packaging_updates","security_stress_fuzz","authenticated_host_transport",
                     "supervisor_apps_restore"}:
            self.assertEqual(known[gate]["status"],"lab_partial")
            self.assertTrue(any(x["release_gate"]==gate for x in self.claims),gate)
        self.assertIn("six dependency",known["third_party_licenses"]["blocked_reason"][0])
    def test_ledger_lists_no_unearned_certifications(self):
        text=(ROOT/"docs/CERTIFICATION_LEDGER.md").read_text()
        self.assertIn("0 / 15",text)
        self.assertIn("Nine gates",text)
        self.assertIn("13 independently observed",text)
        self.assertIn("NOT_PRODUCTION_READY",text)
if __name__=="__main__":unittest.main()
