#!/usr/bin/env python3
"""Adversarial source-license integrity evidence tests (not legal advice)."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
ROOT=Path(__file__).resolve().parent.parent
spec=importlib.util.spec_from_file_location("hars_sources",ROOT/"scripts/audit_cargo_license_sources.py")
audit=importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)

def pkg(root,source=None,name="demo"):
    return {"name":name,"version":"1.0.0","source":source,"manifest_path":str(root/"Cargo.toml")}

class SourceTextEvidence(unittest.TestCase):
    def test_actual_resolved_metadata_coverage(self):
        import os
        path=Path(os.environ.get("HARS_CARGO_LICENSE_METADATA","/tmp/hars-p11b-verify-metadata.json"))
        self.assertTrue(path.is_file())
        r=audit.report(json.loads(path.read_text()))
        self.assertEqual(r["packages_total"],227)
        self.assertEqual(r["external_packages"],219)
        self.assertEqual(r["external_with_source_license_files"],213)
        self.assertFalse(r["third_party_licenses_gate_certified"])
    def test_actual_license_sha256_and_no_absolute_paths(self):
        with tempfile.TemporaryDirectory() as t:
            root=Path(t)
            (root/"LICENSE-MIT").write_bytes(b"fixture-license-contents")
            (root/"Cargo.toml").write_text("[package]\n")
            p=pkg(root,"registry+https://github.com/rust-lang/crates.io-index")
            r=audit.report({"packages":[p]})
            row=r["packages"][0]
            self.assertEqual(row["source_type"],"registry")
            self.assertEqual(len(row["license_and_notice_files"][0]["sha256"]),64)
            self.assertNotIn(t,json.dumps(r))
            self.assertEqual(r["external_with_source_license_files"],1)
    def test_git_ancestor_is_recognized_and_registry_parent_not_inherited(self):
        with tempfile.TemporaryDirectory() as t:
            root=Path(t)
            sub=root/"crates"/"demo"
            sub.mkdir(parents=True)
            (sub/"Cargo.toml").write_text("")
            (root/"LICENSE").write_text("MIT")
            gitpkg=pkg(sub,"git+https://example.com/repo?rev=abc#abc")
            r=audit.report({"packages":[gitpkg]})
            self.assertTrue(r["packages"][0]["from_git_checkout_root"])
            self.assertEqual(r["external_with_source_license_files"],1)
            regpkg=pkg(sub,"registry+https://github.com/rust-lang/crates.io-index")
            r=audit.report({"packages":[regpkg]})
            self.assertEqual(r["external_with_source_license_files"],0)
    def test_symlink_is_not_a_source_of_license_evidence(self):
        with tempfile.TemporaryDirectory() as t:
            root=Path(t)
            (root/"Cargo.toml").write_text("")
            external=Path(t)/"secret"
            external.write_text("not a license")
            (root/"LICENSE").symlink_to(external)
            r=audit.report({"packages":[pkg(root)]})
            self.assertEqual(r["packages"][0]["license_and_notice_files"],[])
    def test_reject_duplicate_manifest_rows_and_unsupported_sources(self):
        with tempfile.TemporaryDirectory() as t:
            root=Path(t)
            p=pkg(root)
            with self.assertRaises(ValueError):audit.report({"packages":[p,p]})
            with self.assertRaises(ValueError):audit.report({"packages":[pkg(root,"git+ssh://unsafe")]})
if __name__=="__main__":unittest.main()
