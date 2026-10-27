#!/usr/bin/env python3
"""P8ea: isolated inode-anchored Supervisor addon.json parsing regressions.

Only local synthetic files. No HAOS, Supervisor tokens, network or production.
"""
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import supervisor_mosquitto_saved_options_lab as reader


def metadata(marker):
    return json.dumps({"version": "1", "system": {},
                       "user": {"options": {"marker": marker}}}).encode()


class DirFdSecurityTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="p8ea-options-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.stage = self.root / "private"
        self.stage.mkdir(mode=0o700)
        self.source = self.stage / "addon.json"
        self.source.write_bytes(metadata("trusted"))
        self.source.chmod(0o600)

    def test_private_real_file_read(self):
        self.assertEqual(reader.read_staged_supervisor_options(self.stage),
                         {"marker": "trusted"})

    def test_parent_directory_swap_cannot_redirect_open(self):
        evil = self.root / "attacker"
        evil.mkdir(mode=0o700)
        (evil / "addon.json").write_bytes(metadata("attacker"))
        (evil / "addon.json").chmod(0o600)
        original_open = os.open
        switched = False

        def swap_at_file_open(path, flags, mode=0o777, *, dir_fd=None):
            nonlocal switched
            if path == "addon.json" and dir_fd is not None and not switched:
                self.stage.rename(self.root / "trusted-original")
                self.stage.symlink_to(evil, target_is_directory=True)
                switched = True
            return original_open(path, flags, mode, dir_fd=dir_fd)

        with patch.object(reader.os, "open", side_effect=swap_at_file_open):
            result = reader.read_staged_supervisor_options(self.stage)
        self.assertTrue(switched)
        self.assertEqual(result, {"marker": "trusted"})

    def test_directory_symlink_refused(self):
        indirect = self.root / "symlink"
        indirect.symlink_to(self.stage, target_is_directory=True)
        with self.assertRaises(reader.SavedOptionsRefused):
            reader.read_staged_supervisor_options(indirect)

    def test_file_symlink_refused(self):
        self.source.unlink()
        self.source.symlink_to(self.root / "untrusted")
        with self.assertRaises(reader.SavedOptionsRefused):
            reader.read_staged_supervisor_options(self.stage)

    def test_hardlink_refused(self):
        os.link(self.source, self.root / "hardlink")
        with self.assertRaises(reader.SavedOptionsRefused):
            reader.read_staged_supervisor_options(self.stage)

    def test_group_or_other_access_refused(self):
        self.stage.chmod(0o750)
        with self.assertRaises(reader.SavedOptionsRefused):
            reader.read_staged_supervisor_options(self.stage)
        self.stage.chmod(0o700)
        self.source.chmod(0o640)
        with self.assertRaises(reader.SavedOptionsRefused):
            reader.read_staged_supervisor_options(self.stage)

    def test_invalid_and_oversized_metadata_refused(self):
        self.source.write_bytes(b'{"user":')
        with self.assertRaises(reader.SavedOptionsRefused):
            reader.read_staged_supervisor_options(self.stage)
        self.source.write_bytes(b' ' * (reader.MAX_ADDON_JSON + 1))
        with self.assertRaises(reader.SavedOptionsRefused):
            reader.read_staged_supervisor_options(self.stage)

    def test_same_size_concurrent_rewrite_after_read_denied(self):
        # Real in-place write between the two fstat calls; size is unchanged.
        trusted = metadata("trusted")
        forged = metadata("forged!")
        self.assertEqual(len(trusted), len(forged))
        original_fstat = os.fstat
        file_fstats = 0

        def rewrite_on_second_file_stat(fd):
            nonlocal file_fstats
            st = original_fstat(fd)
            if os.path.basename(os.readlink(f"/proc/self/fd/{fd}")) == "addon.json":
                file_fstats += 1
                if file_fstats == 2:
                    with open(self.source, "r+b", buffering=0) as out:
                        out.write(forged)
                        out.flush()
                    st = original_fstat(fd)
            return st

        with patch.object(reader.os, "fstat", side_effect=rewrite_on_second_file_stat):
            with self.assertRaises(reader.SavedOptionsRefused):
                reader.read_staged_supervisor_options(self.stage)
        self.assertEqual(file_fstats, 2)
        self.assertEqual(self.source.read_bytes(), forged)

    def test_nested_json_recursion_bomb_refused_not_uncaught(self):
        # Below 128KiB, but deep enough to trigger CPython JSON recursion.
        payload = b'{"version":"1","system":{},"user":{"options":{"nested":' + (
            b"[" * 1500 + b"0" + b"]" * 1500) + b"}}}"
        self.assertLess(len(payload), reader.MAX_ADDON_JSON)
        self.source.write_bytes(payload)
        with self.assertRaisesRegex(reader.SavedOptionsRefused, "nesting"):
            reader.read_staged_supervisor_options(self.stage)

    def test_many_brackets_inside_json_string_are_not_nesting(self):
        self.source.write_bytes(metadata("[" * 3000 + "}" * 3000))
        self.assertEqual(reader.read_staged_supervisor_options(self.stage),
                         {"marker": "[" * 3000 + "}" * 3000})

    def test_fifo_metadata_refused_without_blocking(self):
        self.source.unlink()
        os.mkfifo(self.source, mode=0o600)
        with self.assertRaises(reader.SavedOptionsRefused):
            reader.read_staged_supervisor_options(self.stage)


if __name__ == "__main__":
    unittest.main(verbosity=2)
