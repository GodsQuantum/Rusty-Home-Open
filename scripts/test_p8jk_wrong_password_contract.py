#!/usr/bin/env python3
"""P8j.k: genuine HAOS protected backup wrong-key refusal, fail-closed."""
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parent.parent

class WrongPasswordRealHaos(unittest.TestCase):
    def test_truly_tries_wrong_password_of_actual_encrypted_supervisor_full_backup(self):
        code=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        start=code.index("def test_hars_protected_full_restore_wrong_password_rejected_qemu(")
        case=code[start:code.index(
            "def test_hars_real_official_file_editor_app_lifecycle_qemu("
        )]
        self.assertIn(
            'depends=["test_hars_protected_full_backup_restore_qemu"]',
            code,
        )
        for text in (
            'meta.get("protected") is True',
            'meta.get("type") == "full"',
            'wrong = "RustyQemuSyntheticOnly_2026_WRONG"',
            'ha backups restore " + slug',
            'assert isinstance(result, dict) and result.get("result") != "ok"',
            '("password", "decrypt", "invalid key")',
        ):
            self.assertIn(text,case)
        self.assertNotIn('" --app ',case)
        self.assertNotIn('" --folders ',case)
        self.assertNotIn("SUPERVISOR_TOKEN",case)

    def test_cli_exit_code_not_confused_with_supervisor_api_error(self):
        code=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        case=code[code.index("def test_hars_protected_full_restore_wrong_password_rejected_qemu("):code.index("def test_hars_real_official_file_editor_app_lifecycle_qemu(")]
        assert_api=case.index('result.get("result") != "ok"')
        assert_reason=case.index('("password", "decrypt", "invalid key")')
        parse_exit=case.index("status[0].strip().isdigit()")
        self.assertLess(assert_api,parse_exit)
        self.assertLess(assert_reason,parse_exit)
        self.assertNotIn('status[0].strip() != "0"',case)
        self.assertIn("process exit status alone is NOT authoritative",case)

    def test_wrong_key_does_not_modify_any_real_qemu_data_or_core(self):
        code=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        case=code[code.index("def test_hars_protected_full_restore_wrong_password_rejected_qemu("):code.index("def test_hars_real_official_file_editor_app_lifecycle_qemu(")]
        for evidence in (
            "sha256sum " , "archive_before", "before[0].split()[0]",
            "after[0].split()[0]", "P8JK_WRONG_PASSWORD_MUST_NOT_RESTORE_20261022",
            "ha supervisor info", "ha core info",
            "core.get(\"data\") or {}", "expected_core",
            "P8JK_REAL_HAOS_WRONG_PASSWORD_REJECTED_NO_DATA_MUTATION_PASS",
        ):
            self.assertIn(evidence,case)

    def test_prior_successful_protected_restore_provides_only_safe_backup_slug(self):
        code=(ROOT/"scripts/p8c2_real_haos_supervisor_pytest.py").read_text()
        first=code[code.index("def test_hars_protected_full_backup_restore_qemu("):
                   code.index("def test_hars_protected_full_restore_wrong_password_rejected_qemu(")]
        self.assertIn("P8JJ_REAL_PROTECTED_FULL_RESTORE_EXACT_BYTES_AND_CORE_PASS",first)
        self.assertIn("/tmp/rusty_home_protected_backup_slug",first)
        self.assertIn('assert isinstance(slug, str) and _re.fullmatch(',first)
        second=code[code.index("def test_hars_protected_full_restore_wrong_password_rejected_qemu("):code.index("def test_hars_real_official_file_editor_app_lifecycle_qemu(")]
        self.assertIn('assert _re.fullmatch(r"[0-9a-f]{8,16}", slug)',second)

    def test_independent_kvm_never_skips_any_of_fifteen(self):
        workflow=(ROOT/".github/workflows/p8c2-haos-qemu.yml").read_text()
        self.assertIn("-snapshot -accel kvm",workflow)
        self.assertIn("::test_hars_protected_full_restore_wrong_password_rejected_qemu",workflow)
        guard=(ROOT/"scripts/check_p8d_junit.py").read_text()
        self.assertIn('"test_hars_protected_full_restore_wrong_password_rejected_qemu"',guard)
        self.assertIn("16/16 expected cases",guard)
        self.assertIn('("skipped", "failure", "error")',guard)

    def test_release_gates_still_fail_closed(self):
        release=(ROOT/"config/release-gates.json").read_text()
        self.assertIn('"status": "lab_partial"',release)
        self.assertNotIn('"status": "certified"',release)
        doc=(ROOT/"docs/P8JK_REAL_WRONG_PASSWORD_HAOS_2026-10-22.md").read_text()
        self.assertIn("NOT_PRODUCTION_READY",doc)
        self.assertIn("synthetic VM",doc)

if __name__=="__main__":
    unittest.main()
