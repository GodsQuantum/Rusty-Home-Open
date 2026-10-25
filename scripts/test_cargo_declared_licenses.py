#!/usr/bin/env python3
"""Declared license metadata gate rejects missing external declarations and duplicates."""
from pathlib import Path
import importlib.util
import json
import unittest
import os
ROOT=Path(__file__).resolve().parent.parent
spec=importlib.util.spec_from_file_location("hars_licenses",ROOT/"scripts/audit_cargo_declared_licenses.py")
mod=importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

def metadata(rows):
    return {"packages":rows}
def pkg(name="one",license="MIT",source="registry+https://github.com/rust-lang/crates.io-index"):
    return {"name":name,"version":"1.0.0","license":license,"source":source}

class LicenseEvidence(unittest.TestCase):
    def test_local_real_metadata_declares_external_licenses_but_not_final_compliance(self):
        path=Path(os.environ.get("HARS_CARGO_LICENSE_METADATA","/tmp/hars-p11-metadata.json"))
        self.assertTrue(path.is_file(), "resolved Cargo metadata is required")
        r=mod.report(json.loads(path.read_text()))
        self.assertEqual(r["packages_total"],226)
        self.assertEqual(r["registry_count"],214)
        self.assertEqual(r["git_count"],4)
        self.assertEqual(r["workspace_count"],8)
        self.assertFalse(r["third_party_licenses_gate_certified"])
    def test_missing_registry_or_git_licenses_are_fatal(self):
        for src in ("registry+https://github.com/rust-lang/crates.io-index",
                    "git+https://github.com/owner/repo?rev=" + "a"*40 + "#" + "a"*40):
            with self.assertRaises(ValueError):mod.report(metadata([pkg(license=None,source=src)]))
    def test_owned_workspace_missing_license_is_flagged_not_silenced(self):
        rows=[pkg(license=None,source=None)]
        r=mod.report(metadata(rows))
        self.assertEqual(len(r["unresolved"]),1)
        self.assertEqual(r["unresolved"][0]["issues"],["missing_workspace_license_declaration"])
    def test_legacy_slash_and_duplicates_and_bad_format(self):
        r=mod.report(metadata([pkg(license="MIT/Apache-2.0")]))
        self.assertEqual(r["unresolved"][0]["issues"],["legacy_slash_license_expression_review"])
        with self.assertRaises(ValueError):mod.report(metadata([pkg(),pkg()]))
        with self.assertRaises(ValueError):mod.report(metadata([pkg(license="MIT\nSECRET")]))
    def test_never_records_filesystem_path_or_repository_source(self):
        r=mod.report(metadata([pkg(source="git+https://example.com/owner/repo?rev="+"a"*40)]))
        self.assertNotIn("example.com",json.dumps(r))
        self.assertNotIn("manifest_path",json.dumps(r))
if __name__=="__main__":unittest.main()
