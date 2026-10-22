#!/usr/bin/env python3
"""Check that Supervisor Ingress claim admission remains disabled and non-authorizing."""
from pathlib import Path
import tomllib
import unittest
ROOT = Path(__file__).resolve().parent.parent

class IngressClaim(unittest.TestCase):
    def test_disabled_in_default_and_no_arbitrary_public_peer_acceptor(self):
        cargo=tomllib.loads((ROOT/"crates/supervisor-apps/Cargo.toml").read_text())
        self.assertIn("ingress-identity-lab",cargo["features"])
        self.assertNotIn("ingress-identity-lab",cargo["features"].get("default",[]))
        lib=(ROOT/"crates/supervisor-apps/src/lib.rs").read_text()
        self.assertIn('#[cfg(feature = "ingress-identity-lab")]',lib)
        source=(ROOT/"crates/supervisor-apps/src/ingress_identity_lab.rs").read_text()
        self.assertIn("pub fn ingress_user_claim_from_stream(",source)
        self.assertRegex(source, r"accepted_tcp\s*\.\s*peer_addr\(\)")
        self.assertNotIn("pub fn ingress_user_claim(",source)
        self.assertIn("Ipv4Addr::new(172, 30, 32, 2)",source)
        for bad in ("AuditedPermit", "preview_execute(", "http::Server"):
            self.assertNotIn(bad,source)
    def test_true_loopback_socket_negative_and_upstream_contract_evidence(self):
        tests=(ROOT/"crates/supervisor-apps/tests/p8i_ingress_identity.rs").read_text()
        self.assertIn('TcpListener::bind("127.0.0.1:0")',tests)
        self.assertIn('("X-Forwarded-For", "172.30.32.2")',tests)
        self.assertIn("Err(IngressIdentityError::UntrustedPeer)",tests)
        doc=(ROOT/"docs/P8I_INGRESS_IDENTITY_ADMISSION_LAB_2026-10-21.md").read_text()
        self.assertIn("developers.home-assistant.io/docs/apps/presentation/",doc)
        self.assertIn("github.com/home-assistant/supervisor/blob/main/supervisor/api/ingress.py",doc)
        self.assertIn("NOT_PRODUCTION_READY",doc)

if __name__ == "__main__":
    unittest.main()
