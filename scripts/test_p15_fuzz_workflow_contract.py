#!/usr/bin/env python3
"""Prove instrumented fuzzing remains isolated from runtime/production gates."""
from pathlib import Path
import json
import tomllib
import unittest
ROOT=Path(__file__).resolve().parent.parent
class FuzzIsolation(unittest.TestCase):
    def test_real_binary_target_and_isolated_workspace(self):
        fuzz=tomllib.loads((ROOT/"fuzz/Cargo.toml").read_text())
        self.assertIn("workspace",fuzz)
        self.assertEqual(fuzz["package"]["publish"],False)
        self.assertEqual(fuzz["dependencies"]["libfuzzer-sys"],"=0.4.13")
        target=(ROOT/"fuzz/fuzz_targets/core_user_snapshot.rs").read_text()
        self.assertIn("libfuzzer_sys::fuzz_target",target)
        self.assertIn("CoreUserSnapshot::parse_fixture",target)
        self.assertNotIn("SUPERVISOR_TOKEN",target)
        self.assertNotIn("TcpListener",target)
    def test_ci_nightly_and_sanitizer_scope_fixed(self):
        ci=(ROOT/".github/workflows/fuzz.yml").read_text()
        for expected in ("rustup toolchain install nightly-2026-10-15",
                         "cargo install cargo-fuzz --version 0.13.2 --locked",
                         "fuzz run core_user_snapshot",
                         "-max_total_time=45", "-rss_limit_mb=2048",
                         "contents: read", "pull_request:", "timeout-minutes: 25"):
            self.assertIn(expected,ci)
        self.assertNotIn("pull_request_target:",ci)
        self.assertNotIn("deployment_authorized: true",ci)
    def test_real_ingress_http_parser_fuzzed_with_synthetic_corpus(self):
        target=(ROOT/"fuzz/fuzz_targets/ingress_http.rs").read_text()
        cargo=tomllib.loads((ROOT/"fuzz/Cargo.toml").read_text())
        assert any(x["name"]=="ingress_http" for x in cargo["bin"])
        self.assertIn('src/ingress.rs',target)
        self.assertIn('verified_route(wire, trusted)',target)
        self.assertIn('verified_route(wire, untrusted)',target)
        self.assertNotIn("TcpListener",target)
        self.assertNotIn("SUPERVISOR_TOKEN",target)
        ci=(ROOT/".github/workflows/fuzz.yml").read_text()
        self.assertIn("fuzz run ingress_http",ci)
        self.assertIn('crates/control-plane/src/ingress.rs',ci)
        corpus=list((ROOT/"fuzz/corpus/ingress_http").glob("*.bin"))
        self.assertEqual(len(corpus),6)
        for p in corpus:
            self.assertLess(p.stat().st_size,4096)
            self.assertNotIn(b"SUPERVISOR_TOKEN",p.read_bytes())
            self.assertNotIn(b"real-person",p.read_bytes())
        gitignore=(ROOT/"fuzz/.gitignore").read_text()
        self.assertIn("/corpus/ingress_http/*",gitignore)

    def test_release_still_blocked_and_no_real_personal_ids_in_corpus(self):
        data=json.loads((ROOT/"config/release-gates.json").read_text())
        item=next(g for g in data["required_gates"] if g["id"]=="security_stress_fuzz")
        self.assertNotEqual(item["status"],"certified")
        declared = list((ROOT/"fuzz/corpus/core_user_snapshot").glob("*.json"))
        self.assertEqual(len(declared), 4)
        for p in declared:
            self.assertLess(p.stat().st_size,4096)
            contents=p.read_bytes()
            self.assertNotIn(b"SUPERVISOR_TOKEN",contents)
            self.assertNotIn(b"real-person",contents)
        # Generated libFuzzer binary corpus never belongs in source control.
        ignore=(ROOT/"fuzz/.gitignore").read_text()
        self.assertIn("/corpus/core_user_snapshot/*",ignore)
if __name__=="__main__":
    unittest.main()
