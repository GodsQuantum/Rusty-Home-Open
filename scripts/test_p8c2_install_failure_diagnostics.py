"""Offline negative redaction and zero-cost success-path test for QEMU helper."""
import ast
import contextlib
import io
from pathlib import Path

src = Path(__file__).with_name("p8c2_real_haos_supervisor_pytest.py").read_text()
tree = ast.parse(src)
func = next(x for x in tree.body if isinstance(x, ast.FunctionDef) and x.name == "_hars_emit_safe_install_failure")
scope = {}
exec(compile(ast.Module(body=[func],type_ignores=[]), "helper", "exec"), scope)
diag = scope["_hars_emit_safe_install_failure"]

class FakeDisposableGuest:
    calls = 0
    def run_check(self, command, timeout=30):
        assert command == "ha supervisor logs --no-progress 2>&1 | tail -n 120"
        assert timeout <= 40
        self.calls += 1
        return [
            "INFO healthy",
            "ERROR docker pull failed authorization: verysecretimagekey",
            "ERROR network timeout bearer abc.def.ghi",
            "WARNING irrelevant",
        ]

shell = FakeDisposableGuest()
out = io.StringIO()
with contextlib.redirect_stdout(out):
    diag(shell, {"result": "ok"}, "core_ssh")
assert not out.getvalue() and shell.calls == 0
with contextlib.redirect_stdout(out):
    diag(shell, {"result": "error", "message": "Supervisor error api_token: rawtoken123 password=plaintext-secret"}, "core_ssh")
result = out.getvalue()
assert shell.calls == 1
for secret in ("rawtoken123", "plaintext-secret", "verysecretimagekey", "abc.def.ghi"):
    assert secret not in result, f"secret leaked: {secret}"
assert "QEMU_SUPERVISOR_MESSAGE" in result and "QEMU_SUPERVISOR_LOG" in result
assert result.count("QEMU_SUPERVISOR_LOG ") <= 10 and len(result) < 3000
print("PASS: fail-only QEMU diagnostics bounded and redacted; no change to passing path")
