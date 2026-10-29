"""Unit test for disposable Supervisor plugin convergence and auto-update helper in p8c2."""
import ast
from pathlib import Path

src = Path(__file__).with_name("p8c2_real_haos_supervisor_pytest.py").read_text()
tree = ast.parse(src)

needed = {
    "_plugin_is_current",
    "_ensure_disposable_plugins_current",
    "_prepare_disposable_supervisor_app_environment",
}
funcs = [x for x in tree.body if isinstance(x, ast.FunctionDef) and x.name in needed]
assert len(funcs) == 3, f"Expected 3 functions, found {[f.name for f in funcs]}"

scope = {}
exec(compile(ast.Module(body=funcs, type_ignores=[]), "plugin_helpers", "exec"), scope)

plugin_is_current = scope["_plugin_is_current"]
ensure_plugins = scope["_ensure_disposable_plugins_current"]
prepare_env = scope["_prepare_disposable_supervisor_app_environment"]

# 1. Test _plugin_is_current
assert plugin_is_current({"result": "ok", "data": {"version": "1.0", "version_latest": "1.0"}}, "dns")
assert not plugin_is_current({"result": "ok", "data": {"version": "1.0", "version_latest": "2.0"}}, "dns")

# 1b. Test _ensure_disposable_plugins_current
# Path A: already current (no-update path)
calls_no_up = []
def fake_shell_already_current(cmd, timeout=90):
    calls_no_up.append(cmd)
    return {"result": "ok", "data": {"version": "1.0.0", "version_latest": "1.0.0"}}

ensure_plugins(fake_shell_already_current, lambda s: None)
assert all("info" in c for c in calls_no_up)
assert not any("update" in c for c in calls_no_up)

# Path B: update required (converges after 1 update)
calls_up = []
plugin_versions = {
    "dns": {"version": "1.0.0", "version_latest": "1.0.1"},
    "audio": {"version": "2.0.0", "version_latest": "2.0.0"},
    "multicast": {"version": "3.0.0", "version_latest": "3.0.0"},
    "cli": {"version": "4.0.0", "version_latest": "4.0.0"},
}
def fake_shell_update_required(cmd, timeout=90):
    calls_up.append(cmd)
    for p in ("dns", "audio", "multicast", "cli"):
        if f"ha {p} info" in cmd:
            return {"result": "ok", "data": dict(plugin_versions[p])}
        if f"ha {p} update" in cmd:
            plugin_versions[p]["version"] = plugin_versions[p]["version_latest"]
            return {"result": "ok"}
    return {"result": "ok"}

ensure_plugins(fake_shell_update_required, lambda s: None)
assert any("ha dns update" in c for c in calls_up)
assert not any("ha audio update" in c for c in calls_up)
assert plugin_versions["dns"]["version"] == "1.0.1"

# 2. Test _prepare_disposable_supervisor_app_environment with auto_update already true
calls = []
def fake_shell_json_ok(cmd, timeout=90):
    calls.append(cmd)
    if "supervisor info" in cmd:
        return {"result": "ok", "data": {"auto_update": True}}
    if "info" in cmd:
        return {"result": "ok", "data": {"version": "1.0", "version_latest": "1.0"}}
    return {"result": "ok"}

prepare_env(fake_shell_json_ok, lambda s: None)
assert any("supervisor info" in c for c in calls)
assert not any("options --auto-update" in c for c in calls)

# 3. Test _prepare_disposable_supervisor_app_environment with auto_update initially false
calls2 = []
state = {"auto_update": False}
def fake_shell_json_converge(cmd, timeout=90):
    calls2.append(cmd)
    if "supervisor info" in cmd:
        return {"result": "ok", "data": {"auto_update": state["auto_update"]}}
    if "options --auto-update" in cmd:
        state["auto_update"] = True
        return {"result": "ok"}
    if "supervisor reload" in cmd:
        return {"result": "ok"}
    if "info" in cmd:
        return {"result": "ok", "data": {"version": "1.0", "version_latest": "1.0"}}
    return {"result": "ok"}

prepare_env(fake_shell_json_converge, lambda s: None)
assert any("options --auto-update" in c for c in calls2)
assert any("supervisor reload" in c for c in calls2)

print("PASS: p8c2 plugin convergence and auto-update environment unit tests passed")
