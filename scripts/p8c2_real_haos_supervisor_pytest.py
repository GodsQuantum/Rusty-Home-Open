
# APPENDED TO: upstream home-assistant/operating-system/tests/supervisor_test/test_supervisor.py
# Upstream setup/fixtures come from official pinned HAOS integration test harness.
# Only runs in an ephemeral hosted QEMU guest; never against sample lab host synthetic VM.

def _hars_emit_safe_install_failure(shell, result, slug):
    """Bounded and redacted diagnostics from disposable HAOS QEMU only."""
    import re as _re

    if result.get("result") == "ok":
        return

    def scrub(value):
        raw = str(value or "")[:1800]
        raw = _re.sub(r"(?i)(bearer\s+)[A-Za-z0-9._~+/=-]+", r"\1[REDACTED]", raw)
        raw = _re.sub(
            r"(?i)((?:access_?token|refresh_?token|api_?token|password|secret|authorization|credential|apikey|api_key)\s*[:=]\s*)[^\s,;]+",
            r"\1[REDACTED]", raw,
        )
        return raw[:260]

    print("QEMU_SUPERVISOR_INSTALL_FAILURE", slug)
    print("QEMU_SUPERVISOR_RESULT", scrub(result.get("result")))
    print("QEMU_SUPERVISOR_MESSAGE", scrub(result.get("message")))
    try:
        rows = shell.run_check(
            "ha supervisor logs --no-progress 2>&1 | tail -n 120",
            timeout=40,
        )
        tail = rows if isinstance(rows, list) else str(rows).splitlines()
        selected = [
            scrub(line) for line in tail
            if _re.search(r"(?i)error|fail|denied|timeout|image|docker|storage|no.space|out.of.space|network|OCI|certificate|unavailable", str(line))
        ]
        for line in selected[-10:]:
            print("QEMU_SUPERVISOR_LOG", line)
    except Exception as exc:
        print("QEMU_SUPERVISOR_LOG_UNAVAILABLE", scrub(type(exc).__name__))


def _plugin_is_current(info, name):
    """Require a genuine version match, not merely Supervisor auto_update=True."""
    if not isinstance(info, dict) or info.get("result") != "ok":
        raise AssertionError("Official disposable plugin info unavailable: " + name)
    data = info.get("data")
    if not isinstance(data, dict):
        raise AssertionError("Official disposable plugin metadata missing: " + name)
    version, latest = data.get("version"), data.get("version_latest")
    if not (isinstance(version, str) and version.strip()
            and isinstance(latest, str) and latest.strip()):
        raise AssertionError("Official disposable plugin versions unavailable: " + name)
    return version == latest


def _ensure_disposable_plugins_current(shell_json, sleeper):
    """Only upstream HA CLI updates in ephemeral QEMU; never a real host."""
    for name in ("dns", "audio", "multicast", "cli"):
        query = "ha " + name + " info --no-progress --raw-json"
        if _plugin_is_current(shell_json(query, timeout=90), name):
            continue
        try:
            response = shell_json(
                "ha " + name + " update --no-progress --raw-json", timeout=420
            )
            if response.get("result") != "ok":
                raise AssertionError("Official plugin update denied: " + name)
        except Exception:
            pass
        for attempt in range(36):
            try:
                if _plugin_is_current(shell_json(query, timeout=90), name):
                    break
            except Exception:
                pass
            if attempt < 35:
                sleeper(3)
        else:
            raise AssertionError("Official disposable plugin did not converge: " + name)


def _prepare_disposable_supervisor_app_environment(shell_json, sleeper):
    """Enable auto-update and converge disposable plugins for Supervisor App installs."""
    supervisor = shell_json("ha supervisor info --no-progress --raw-json", timeout=90)
    assert supervisor.get("result") == "ok", "Supervisor info query failed"
    state = supervisor.get("data") or {}
    assert isinstance(state.get("auto_update"), bool), "Supervisor auto-update state unavailable"
    if not state["auto_update"]:
        options = shell_json(
            "ha supervisor options --auto-update --no-progress --raw-json",
            timeout=120,
        )
        assert options.get("result") == "ok", "Failed to enable Supervisor auto-update in fixture"
        reloaded = shell_json("ha supervisor reload --no-progress --raw-json", timeout=120)
        assert reloaded.get("result") == "ok", "Failed to reload Supervisor after auto-update change"
        for _ in range(30):
            sleeper(1)
            info = shell_json("ha supervisor info --no-progress --raw-json", timeout=90)
            if (info.get("data") or {}).get("auto_update") is True:
                break
        else:
            raise AssertionError("Supervisor auto_update failed to reflect as True in disposable QEMU")
    _ensure_disposable_plugins_current(shell_json, sleeper)
