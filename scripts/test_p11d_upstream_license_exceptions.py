#!/usr/bin/env python3
"""Pinned upstream references do not close third-party license gate."""
import json
import unittest
from pathlib import Path
R=Path(__file__).resolve().parent.parent
class Review(unittest.TestCase):
    def test_exact_six_missing_external_packages(self):
        old=json.loads((R/"audits/CARGO_SOURCE_LICENSE_HASHES_2026-10-21.json").read_text())
        new=json.loads((R/"audits/P11D_UPSTREAM_LICENSE_FOLLOWUP_2026-10-21.json").read_text())
        missing={(x["name"],x["version"]) for x in old["unresolved"] if x["source_type"]=="registry"}
        current={(x["name"],x["version"]) for x in new["rows"]}
        self.assertEqual(len(missing),6)
        self.assertEqual(missing,current)
        self.assertFalse(new["production_license_gate_certified"])
    def test_version_refs_and_no_certification_claim(self):
        data=json.loads((R/"audits/P11D_UPSTREAM_LICENSE_FOLLOWUP_2026-10-21.json").read_text())
        for x in data["rows"]:
            self.assertTrue(x["repository"].startswith("https://github.com/"))
            sha=x.get("upstream_commit") or x["annotated_tag_git_sha1"]
            self.assertRegex(sha,r"^[0-9a-f]{40}$")
            self.assertNotEqual(x["provenance"],"certified")
        self.assertGreaterEqual(len(data["remaining"]),4)
    def test_release_guard_is_still_blocked(self):
        doc=json.loads((R/"config/release-gates.json").read_text())
        from check_release_gate import audit
        live=audit(doc)
        license_gate=next(g for g in doc["required_gates"] if g["id"]=="third_party_licenses")
        self.assertTrue(license_gate["status"]!="certified" or license_gate["id"] not in live["blocked_ids"])
        self.assertFalse(live["deployment_authorized"])
if __name__=="__main__":
    unittest.main()
