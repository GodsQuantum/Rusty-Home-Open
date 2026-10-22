#!/usr/bin/env python3
"""P11a Cargo dependency integrity review is fail closed and not a legal signoff."""
import importlib.util
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parent.parent
spec=importlib.util.spec_from_file_location("hars_lock_provenance",ROOT/"scripts/audit_cargo_lock_provenance.py")
module=importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
GOOD_HASH="a"*64
GIT_SHA="b"*40

def render(rows):
    return ("version = 4\n"+rows).encode()

class ProvenanceGate(unittest.TestCase):
    def test_repository_real_lock_has_all_sources_pinned(self):
        report=module.inventory((ROOT/"Cargo.lock").read_bytes())
        self.assertGreater(report["packages_total"],100)
        self.assertEqual(report["checksum_git_pin_validation"],"passed")
        self.assertFalse(report["release_gate_third_party_licenses_certified"])
        self.assertFalse(report["sbom_complete"])
        self.assertEqual(report["packages_total"],sum(report["counts"].values()))
        self.assertEqual(report["packages"],sorted(report["packages"],key=lambda x:(x["name"],x["version"],x["source_type"])))
    def test_rejects_registry_missing_corrupt_sha_or_duplicate(self):
        a='[[package]]\nname = "a"\nversion = "1.0.0"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\n'
        for item in (a, a+'checksum = "invalid"\n', a+f'checksum = "{GOOD_HASH}"\n'+a+f'checksum = "{GOOD_HASH}"\n'):
            with self.assertRaises(ValueError):module.inventory(render(item))
        assert module.inventory(render(a+f'checksum = "{GOOD_HASH}"\n'))["counts"]["registry"]==1
        credentialed = a.replace("https://github.com", "https://user:secret@github.com")
        with self.assertRaises(ValueError):
            module.inventory(render(credentialed+f'checksum = "{GOOD_HASH}"\n'))
    def test_git_revisions_need_matching_full_40_char_shas(self):
        prefix='[[package]]\nname = "moli"\nversion = "1.0.0"\n'
        for src in (
            f"git+https://github.com/example/driver?branch=main#{GIT_SHA}",
            "git+https://github.com/example/driver?rev=abcdef#abcdef",
            f"git+https://github.com/example/driver?rev={GIT_SHA}#"+("c"*40),
            f"git+https://user:secret@github.com/example/driver?rev={GIT_SHA}#{GIT_SHA}",
            "git+ssh://git@github.com/foo/bar#"+GIT_SHA,
        ):
            with self.assertRaises(ValueError):
                module.inventory(render(prefix+f'source = "{src}"\n'))
        valid=prefix+f'source = "git+https://github.com/example/driver?rev={GIT_SHA}#{GIT_SHA}"\n'
        self.assertEqual(module.inventory(render(valid))["counts"]["git"],1)
    def test_workspace_crate_not_misrepresented_as_external(self):
        local='[[package]]\nname = "my-crate"\nversion = "0.1.0"\n'
        r=module.inventory(render(local))
        self.assertEqual(r["counts"]["workspace"],1)
        self.assertFalse(r["transitive_license_review_complete"])

if __name__=="__main__":unittest.main()
