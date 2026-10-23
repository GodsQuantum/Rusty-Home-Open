#!/usr/bin/env python3
"""Strict regression for HAOS serial-padding output on Supervisor restart."""
import unittest
from p8c2_serial_invocation import parse_systemd_invocation

class SupervisorInvocationProof(unittest.TestCase):
    def test_valid_literal_and_qemu_tilde_padding(self):
        raw = "0ad3b204b08947efb831378e23a71990"
        self.assertEqual(parse_systemd_invocation(raw), raw)
        self.assertEqual(parse_systemd_invocation(raw + "~" * 21 + "\r\n"), raw)
        self.assertEqual(parse_systemd_invocation(raw.upper() + "~" * 10), raw)
    def test_invalid_ambiguous_or_truncated_identifiers_fail_closed(self):
        raw = "0ad3b204b08947efb831378e23a71990"
        for bad in ("", "~" * 32, raw[:-1], raw + "0", raw + " evil",
                    raw + "~" * 3 + "A", "trace:" + raw, raw + "\x1b[0m",
                    raw + ("~" * 260)):
            with self.subTest(bad=repr(bad)):
                with self.assertRaises(ValueError):
                    parse_systemd_invocation(bad)

if __name__ == "__main__":
    unittest.main()
