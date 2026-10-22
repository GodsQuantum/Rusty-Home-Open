#!/usr/bin/env python3
"""P4 epoch one-use and time-bounded WS trust regression contract."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parent.parent

class LeaseAndEpoch(unittest.TestCase):
    def test_reauthentication_required_after_any_revocation(self):
        code=(ROOT/"crates/ha-compat/src/session.rs").read_text()
        self.assertIn("snapshot_installed: bool", code)
        self.assertIn("if self.snapshot_installed {", code)
        self.assertIn("self.snapshot_installed = true;", code)
        self.assertIn("self.snapshot_installed = false;", code)
        self.assertIn("pub fn lost_connection(", code)
        self.assertIn("connection generation already used: reauthentication required",code)
        self.assertIn("revoked_epoch_must_never_install_a_second_snapshot_without_reconnecting",code)
    def test_snapshot_freshness_and_idle_wait_are_bounded(self):
        code=(ROOT/"crates/ha-compat/src/session.rs").read_text()
        self.assertIn("SNAPSHOT_MAX_AGE: Duration = Duration::from_secs(30)",code)
        self.assertIn("snapshot_at: Option<Instant>",code)
        for required in (
            "self.ensure_snapshot_fresh()?",
            "self.snapshot_at = Some(Instant::now());",
            "tokio::time::timeout(wait, relay.recv()).await",
            "self.snapshot_at = None;",
            "stale_snapshot_lease_blocks_cached_reads_even_if_socket_is_open",
            "idle_open_stream_cannot_block_past_snapshot_lease_without_resync",
        ):
            self.assertIn(required,code)
    def test_lab_only_and_release_gate_kept_blocked(self):
        src=(ROOT/"crates/ha-compat/src/lib.rs").read_text()
        self.assertIn("explicit loopback WebSocket required",src)
        self.assertNotIn("call_service",src.split("pub mod session;")[1])
        doc=(ROOT/"docs/P4_WS_EPOCH_LEASE_2026-10-22.md").read_text()
        self.assertIn("NOT_PRODUCTION_READY",doc)
        self.assertIn("30 secondes",doc)
        self.assertIn("ha_ws_reconnection",doc)
if __name__=="__main__":unittest.main()
