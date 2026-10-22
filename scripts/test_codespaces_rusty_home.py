#!/usr/bin/env python3
"""Static, portable Codespaces and Rusty Home branding contract."""
import json
from pathlib import Path
import unittest
import tomllib

ROOT=Path(__file__).resolve().parent.parent
class CodespacesConfig(unittest.TestCase):
    def test_devcontainer_machine_and_private_network(self):
        data=json.loads((ROOT/".devcontainer/devcontainer.json").read_text())
        self.assertIn("devcontainers/rust",data["image"])
        self.assertGreaterEqual(data["hostRequirements"]["cpus"],4)
        self.assertIn("16gb",data["hostRequirements"]["memory"].lower())
        self.assertEqual(data["forwardPorts"],[])
        self.assertEqual(data["portsAttributes"]["18790"]["onAutoForward"],"ignore")
        self.assertEqual(data["postCreateCommand"],"bash .devcontainer/bootstrap.sh")
        self.assertIn("ghcr.io/devcontainers/features/sshd:1",data["features"])
        self.assertIn("ghcr.io/devcontainers/features/github-cli:1",data["features"])
        self.assertNotIn("secrets",data)
        self.assertNotIn("mounts",data)
    def test_bootstrap_never_deploys_or_clears_backups(self):
        text=(ROOT/".devcontainer/bootstrap.sh").read_text()
        self.assertIn("cargo fetch --locked",text)
        self.assertIn("scripts/check_release_gate.py --expect-blocked",text)
        for bad in ("docker run", "pct ", "qm ", "rm -rf", "git clean", "ssh ", "/srv/storage"):
            self.assertNotIn(bad,text)
        verify=(ROOT/".devcontainer/verify.sh").read_text()
        self.assertIn("cargo test --workspace --locked",verify)
        self.assertIn("cargo clippy --workspace",verify)
    def test_manifests_use_canonical_repo_name_and_never_expand_privileges(self):
        for slug in ("hars_rust_probe","hars_core_ws_probe","hars_version_fixture"):
            s=(ROOT/f"packaging/supervisor-app/{slug}/config.yaml").read_text()
            self.assertIn('url: "https://github.com/GodsQuantum/Rusty-Home-Open"',s)
            self.assertIn("Rusty Home",s.splitlines()[0])
            self.assertIn("protected: true",s)
            self.assertIn("docker_api: false",s)
            self.assertIn("host_network: false",s)
        self.assertIn("homeassistant_api: false",(ROOT/"packaging/supervisor-app/hars_rust_probe/config.yaml").read_text())
    def test_release_gate_still_requires_independent_evidence(self):
        data=json.loads((ROOT/"config/release-gates.json").read_text())
        self.assertEqual(len(data["required_gates"]),15)
        self.assertTrue(all(g["status"] != "certified" for g in data["required_gates"]))
        doc=(ROOT/"docs/HANDOFF_CODESPACES_2026-10-21.md").read_text()
        self.assertIn("NOT_PRODUCTION_READY",doc)
        self.assertIn("2e3705252998bfa2bd455b4ea81de57d00961a02",doc)
if __name__=="__main__":
    unittest.main()
