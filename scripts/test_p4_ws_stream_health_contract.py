#!/usr/bin/env python3
"""Read-side websocket failure-invalidation contract in the HA Rust lab."""
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parent.parent
class StreamSafety(unittest.TestCase):
    def test_event_relay_and_safe_read_guards_exist(self):
        lib=(ROOT/"crates/ha-compat/src/lib.rs").read_text()
        sess=(ROOT/"crates/ha-compat/src/session.rs").read_text()
        for token in ("pub fn check_live(", "pub fn is_closed_or_overflowed(", "self.rx.is_closed()", "self.overflow.load("):
            self.assertIn(token, lib)
        for token in ("pub fn read_live(", "pub fn try_recv_event(", "pub fn transport_failed(", "relay.check_live()", "self.mirror = None", "self.try_recv_event(epoch, relay)?"):
            self.assertIn(token, sess)
        for name in (
            "dropped_event_source_invalidates_cache_without_waiting_for_next_event",
            "overflow_cannot_be_ignored_by_cached_read",
            "nonblocking_receive_requires_same_generation_and_never_revives_stale_cache",
            "live_read_drains_queued_state_change_before_returning_cached_value",
            "authentic_ws_resync_never_reuses_old_generation",
        ):
            self.assertIn(name, sess)
    def test_release_remains_blocked_and_limitations_explicit(self):
        doc=(ROOT/"docs/P4_WS_STREAM_HEALTH_LAB_2026-10-21.md").read_text()
        self.assertIn("ne fournit pas de numéro de séquence durable",doc)
        self.assertIn("BLOCKED",doc)
        self.assertIn("subscribe_entities",doc)
        self.assertIn("localhost",doc)
if __name__=="__main__":
    unittest.main()
