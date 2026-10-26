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
    def test_fifteen_require_proofs_one_by_one_and_never_authorize_deployment(self):
        state=audit(self.manifest)
        self.assertEqual(state["total_gates"],15)
        self.assertEqual(state["blocking_gates"]+state["progress"]["certified"],15)
        self.assertFalse(state["deployment_authorized"])
        self.assertFalse(state["independent_release_signoff"])
        self.assertEqual(sum(x["status"]=="lab_partial" for x in self.rows),state["progress"]["lab_partial"])
        self.assertEqual(sum(x["status"]=="blocked" for x in self.rows),state["progress"]["blocked"])
        self.assertTrue(all(x["status"]!="certified" or x["id"] not in state["blocked_ids"] for x in self.rows))
    def test_real_evidence_never_waives_production_acceptance(self):
        self.assertEqual(len(self.claims),15)
        known={x["id"]:x for x in self.rows}
        self.assertEqual(set(known),{x["id"] for x in self.rows})
        self.assertTrue(all(x["blocked_reason"] and len(x["blocked_reason"][0])>80
                            for x in self.rows if x["status"]!="certified"))
        self.assertTrue(all("Synthetic public alpha:" not in x["blocked_reason"][0]
                            for x in self.rows))
        for gate in {"packaging_updates","security_stress_fuzz","authenticated_host_transport",
                     "supervisor_apps_restore"}:
            self.assertIn(known[gate]["status"],("lab_partial","certified"))
            self.assertTrue(any(x["release_gate"]==gate for x in self.claims),gate)
        if known["third_party_licenses"]["status"]!="certified":
            self.assertIn("Six external",known["third_party_licenses"]["blocked_reason"][0])
            self.assertIn("Sigstore-attested",known["third_party_licenses"]["blocked_reason"][0])
        claims={x["id"]:x for x in self.claims}
        self.assertEqual(claims["fuzz_four_native_security_surfaces"]["run_id"],40026397117)
        self.assertEqual(claims["github_oidc_cargo_source_sbom"]["run_id"],40027460088)
    def test_ledger_lists_no_unearned_certifications(self):
        text=(ROOT/"docs/CERTIFICATION_LEDGER.md").read_text()
        state=audit(self.manifest)
        self.assertIn(f"{state['progress']['certified']} / 15",text)
        self.assertIn("15 independently observed",text)
        self.assertIn("NOT_PRODUCTION_READY" if audit(self.manifest)["blocking_gates"] else "technical_evidence_complete_pending_release_signoff",text)
if __name__=="__main__":unittest.main()
