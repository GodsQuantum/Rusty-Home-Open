"""P8du: two genuine Supervisor Apps concurrently installed in one disposable HAOS18.3 guest.

Each App gets an official selected-App-only archive sent to a separate private
loopback-only relay. All payloads are fictional and nonsecret; never VM100.
"""
import base64
import hashlib
import json
import re
from time import sleep
import pytest
from p8do_official_mosquitto_qemu_pytest import _ensure_disposable_plugins_current

APP_MARKERS = {
    "core_configurator": ("rusty_home_p8du_editor_marker.txt", b"P8DU_REAL_EDITOR_SIMULTANEOUS_NONSECRET_20261027\\n", 18674),
    "core_mosquitto": ("rusty_home_p8du_mqtt_marker.txt", b"P8DU_REAL_MQTT_SIMULTANEOUS_NONSECRET_20261027\\n", 18675),
}

@pytest.mark.dependency(depends=["test_start_supervisor"])
@pytest.mark.timeout(2000)
def test_hars_real_two_simultaneous_app_only_partial_exports(shell, shell_json):
    installed = []
    backups = []
    try:
        state = shell_json("ha supervisor info --no-progress --raw-json", timeout=90)
        assert state.get("result") == "ok"
        assert isinstance((state.get("data") or {}).get("auto_update"), bool)
        if not state["data"]["auto_update"]:
            assert shell_json("ha supervisor options --auto-update --no-progress --raw-json", timeout=120).get("result") == "ok"
            assert shell_json("ha supervisor reload --no-progress --raw-json", timeout=200).get("result") == "ok"
            for attempt in range(24):
                try:
                    status = shell_json("ha supervisor info --no-progress --raw-json", timeout=90)
                    if status.get("result") == "ok" and (status.get("data") or {}).get("auto_update") is True:
                        break
                except Exception:
                    pass
                sleep(3)
            else:
                raise AssertionError("Official disposable Supervisor auto-update never became active")
        _ensure_disposable_plugins_current(shell_json, sleep)
        assert shell_json("ha core info --no-progress --raw-json", timeout=90).get("result") == "ok"

        for slug in APP_MARKERS:
            available = None
            for reload_attempt in range(2):
                for poll in range(13):
                    try:
                        result = shell_json("ha apps info " + slug + " --no-progress --raw-json", timeout=80)
                    except Exception:
                        result = None
                    data = (result or {}).get("data") or {}
                    if (result and result.get("result") == "ok" and data.get("slug") == slug
                        and isinstance(data.get("version_latest"), str) and data["version_latest"].strip()
                        and data.get("repository") in ("core", None) and data.get("state") == "unknown"):
                        available = data
                        break
                    if poll < 12:
                        sleep(2)
                if available is not None:
                    break
                if reload_attempt == 0:
                    try:
                        shell_json("ha store reload --no-progress --raw-json", timeout=150)
                    except Exception:
                        pass
            assert available is not None, "Official Store App missing: " + slug
            stdout, stderr, exit_code = shell.run(
                "ha apps install " + slug + " --no-progress --raw-json", timeout=450)
            if exit_code != 0:
                try:
                    observed = shell_json("ha apps info " + slug + " --no-progress --raw-json", timeout=90)
                    if (observed.get("data") or {}).get("state") not in ("unknown", None):
                        installed.append(slug)
                except Exception:
                    pass
                raise AssertionError("Official App install returned nonzero for " + slug)
            result = json.loads("\n".join(stdout))
            assert result.get("result") == "ok", "Official Supervisor install rejected: " + slug
            installed.append(slug)
            info = shell_json("ha apps info " + slug + " --no-progress --raw-json", timeout=90)
            assert info.get("result") == "ok"
            assert (info.get("data") or {}).get("state") in ("started", "stopped")
            assert (info.get("data") or {}).get("version")

        # Both Apps must coexist BEFORE either official archive is generated.
        for slug in APP_MARKERS:
            info = shell_json("ha apps info " + slug + " --no-progress --raw-json", timeout=90)
            assert (info.get("data") or {}).get("state") in ("started", "stopped")
        print("P8DU_REAL_HAOS_TWO_OFFICIAL_APPS_SIMULTANEOUSLY_INSTALLED_PASS", flush=True)

        boot = shell.run_check("cat /proc/sys/kernel/random/boot_id")[0].strip()
        assert re.fullmatch(r"[0-9a-f-]{36}", boot)
        core = shell_json("ha core info --no-progress --raw-json", timeout=95)
        version = (core.get("data") or {}).get("version")
        assert core.get("result") == "ok" and re.fullmatch(r"[0-9A-Za-z_.+-]{1,64}", version or "")

        for slug, (filename, payload, port) in APP_MARKERS.items():
            roots = ("/mnt/data/supervisor/apps/data/" + slug,
                     "/mnt/data/supervisor/addons/data/" + slug)
            found = [root for root in roots if shell.run_check(
                "test -d " + root + " && echo PRESENT || echo ABSENT", timeout=30
            )[0].strip() == "PRESENT"]
            assert len(found) == 1
            marker_path = found[0] + "/" + filename
            encoded = base64.b64encode(payload).decode("ascii")
            assert len(encoded) < 150
            shell.run_check("printf %s " + encoded + " | base64 -d > " + marker_path, timeout=30)
            assert shell.run_check("sha256sum " + marker_path, timeout=30)[0].split()[0] == hashlib.sha256(payload).hexdigest()
            created = shell_json('ha backups new --name "P8DU Two Real Apps ' + slug
                                 + '" --app ' + slug + " --no-progress --raw-json", timeout=460)
            backup_slug = (created.get("data") or {}).get("slug")
            assert created.get("result") == "ok" and re.fullmatch(r"[0-9a-f]{8,16}", backup_slug or "")
            backups.append(backup_slug)
            details = shell_json("ha backups info " + backup_slug + " --no-progress --raw-json", timeout=95)
            meta = details.get("data") or {}
            assert details.get("result") == "ok" and meta.get("type") == "partial" and meta.get("protected") is False
            addons = meta.get("addons")
            assert isinstance(addons, list)
            assert [x.get("slug") if isinstance(x, dict) else x for x in addons] == [slug]
            archive = "/mnt/data/supervisor/backup/" + backup_slug + ".tar"
            assert shell.run_check("test -s " + archive + " && echo PRESENT")[0].strip() == "PRESENT"
            digest = shell.run_check("sha256sum " + archive, timeout=45)[0].split()[0]
            assert re.fullmatch(r"[0-9a-f]{64}", digest)
            transfer = shell.run_check(
                "curl --fail --silent --show-error --max-time 460 "
                "-H 'X-Hars-Slug: " + backup_slug + "' "
                "-H 'X-Hars-Boot: " + boot + "' "
                "-H 'X-Hars-Core: " + version + "' "
                "-T " + archive + " http://10.0.2.2:" + str(port) + "/archive"
                " && echo P8DU_REAL_SELECTED_APP_TRANSFER_DONE", timeout=480)
            assert "P8DU_REAL_SELECTED_APP_TRANSFER_DONE" in transfer
        print("P8DU_TWO_REAL_OFFICIAL_APP_ONLY_PARTIAL_ARCHIVES_SAME_BOOT_PASS", flush=True)
    finally:
        for backup_slug in reversed(backups):
            assert shell_json("ha backups remove " + backup_slug + " --no-progress --raw-json",
                              timeout=180).get("result") == "ok", "Disposable archive cleanup failed"
        for slug in reversed(installed):
            assert shell_json("ha apps uninstall " + slug + " --no-progress --raw-json",
                              timeout=260).get("result") == "ok", "Disposable App cleanup failed"

