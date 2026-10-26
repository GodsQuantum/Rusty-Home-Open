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
        # Fifteen is the number of product gates, NOT a cap on independent
        # laboratory receipts. Preserve the original fifteen immutable IDs.
        self.assertGreaterEqual(len(self.claims),15)
        baseline = ["debian_real_dpkg","apt_signed_tamper_refusal","official_core_rust_bridge","selected_hacs_three","adaptive_media_hacs","epson_snmp_hacs","card_announcements_hacs","native_rust_security_suite","official_product_nonroot","official_marketplace_abi","official_haos_kvm","browser_chromium","security_fuzzer","fuzz_four_native_security_surfaces","github_oidc_cargo_source_sbom"]
        self.assertTrue(set(baseline) <= {x["id"] for x in self.claims})
        mqtt = next(x for x in self.claims if x["id"] == "mqtt5_real_broker_sigkill_stale_quarantine")
        self.assertEqual(mqtt["status"], "verified_lab_only")
        self.assertEqual(mqtt["release_gate"], "native_drivers_services")
        p2j = next(x for x in self.claims if x["id"] == "moli_live_tombstone_same_id_readd")
        self.assertEqual(p2j["status"], "verified_lab_only")
        self.assertEqual(p2j["release_gate"], "native_drivers_services")
        self.assertEqual(p2j["run_id"], 40380159515)
        self.assertEqual(p2j["head_sha"], "7a5944e7e96838fefabd5305dbf415074dec4581")
        p2l = next(x for x in self.claims if x["id"] == "moli_atomic_direct_discovery_batch_refusal")
        self.assertEqual(p2l["status"], "verified_lab_only")
        self.assertEqual(p2l["release_gate"], "native_drivers_services")
        self.assertEqual(p2l["run_id"], 40389728103)
        self.assertEqual(p2l["head_sha"], "6c2d061f251149b1a9afa1424a8971c218bf0b78")
        p2k = next(x for x in self.claims if x["id"] == "mqtt5_live_broker_tombstone_same_id_readd")
        self.assertEqual(p2k["status"], "verified_lab_only")
        self.assertEqual(p2k["release_gate"], "native_drivers_services")
        self.assertEqual(p2k["run_id"], 40397842879)
        self.assertEqual(p2k["head_sha"], "bae1ee85596c7d2004bda91d2281b2a091c20303")
        self.assertEqual(p2k["merged_pr"], 157)
        # Exact CI receipts are laboratory evidence, never release signatures.
        for claim_id, pr, run, head in (
            ("moli_direct_topic_alias_owner_fence", 161, 40404859841, "c0b5cd4c963ca8f9ea1ca5162ec183ff53278909"),
            ("mqtt_redb_durable_topic_identity_collision_refusal", 162, 40406964357, "4837233c4870c6e0604d35a064f6d748c264591f"),
        ):
            item = next(x for x in self.claims if x["id"] == claim_id)
            self.assertEqual(item["release_gate"], "native_drivers_services")
            self.assertEqual(item["status"], "verified_lab_only")
            self.assertEqual(item["merged_pr"], pr)
            self.assertEqual(item["run_id"], run)
            self.assertEqual(item["head_sha"], head)
        p8ca = next(x for x in self.claims if x["id"] == "oci_postgresql_wal_sigkill_transaction_recovery")
        self.assertEqual(p8ca["status"], "verified_lab_only")
        self.assertEqual(p8ca["release_gate"], "supervisor_apps_restore")
        self.assertEqual(p8ca["run_id"], 40415028970)
        self.assertEqual(p8ca["head_sha"], "4299ca4d8c2a7d01fc471ad706b15c44d664f0cc")
        self.assertEqual(p8ca["merged_pr"], 168)
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
