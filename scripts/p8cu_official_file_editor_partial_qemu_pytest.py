"""Official disposable HAOS18.3 real core_configurator PARTIAL App backup.

App is installed through genuine Supervisor and synthetic nonsecret marker
written only to disposable official App data before backup. Never VM100.
"""
import hashlib
import os
from pathlib import Path
import pytest
import re

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
        # One official update POST; never retry blindly if it disconnects CLI.
        try:
            response = shell_json(
                "ha " + name + " update --no-progress --raw-json", timeout=420
            )
            if response.get("result") != "ok":
                raise AssertionError("Official plugin update denied: " + name)
        except Exception:
            # The CLI plugin itself may terminate its own shell session.
            # The independent version GET below is the only acceptance rule.
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


@pytest.mark.dependency(depends=["test_start_supervisor"])
@pytest.mark.timeout(1500)
def test_hars_official_file_editor_app_only_partial_export(shell,shell_json):
    slug="core_configurator"
    marker_name="rusty_home_p8cu_nonsecret_synthetic_marker.txt"
    marker_value=b"P8CU_OFFICIAL_SUPERVISOR_APP_ONLY_PRIVATE_MARKER_20261027\\n"
    marker_hash=hashlib.sha256(marker_value).hexdigest()
    added=False
    backup_slug=None
    try:
        from time import sleep as _sleep
        # Actual Supervisor 2026 policy forbids any App install when plugins
        # are outdated and auto-update=false. The pinned official 18.3
        # disposable QEMU fixture boots with auto-update disabled.
        # Use ONLY upstream Supervisor options and reload within disposable
        # QEMU; never patch Supervisor jobs or change actual user HAOS.
        supervisor=shell_json("ha supervisor info --no-progress --raw-json",timeout=90)
        assert supervisor.get("result")=="ok"
        state=supervisor.get("data") or {}
        assert isinstance(state.get("auto_update"),bool), "Supervisor auto-update state unavailable"
        if not state["auto_update"]:
            options=shell_json(
                "ha supervisor options --auto-update --no-progress --raw-json",
                timeout=120,
            )
            assert options.get("result")=="ok", "Official Supervisor refused fixture-only plugin update policy"
            reloaded=shell_json("ha supervisor reload --no-progress --raw-json",timeout=200)
            assert reloaded.get("result")=="ok", "Official Supervisor fixture reload failed"
            for check in range(24):
                try:
                    current=shell_json("ha supervisor info --no-progress --raw-json",timeout=60)
                except Exception:
                    current=None
                if (current and current.get("result")=="ok"
                    and (current.get("data") or {}).get("auto_update") is True):
                    break
                if check<23:_sleep(3)
            else:
                raise AssertionError("Upstream Supervisor plugin auto-update flag never became true")
        _ensure_disposable_plugins_current(shell_json, _sleep)
        print("P8CU_DISPOSABLE_HAOS_UPSTREAM_SUPERVISOR_PLUGIN_POLICY_READY_PASS")
        # Official Store's catalogue populates asynchronously on a fresh
        # Supervisor. Require the ACTUAL fixed public App version and
        # repository in a bounded observed GET, not merely a successful POST.
        # This mirrors the established official p8c2 full HAOS Store harness.
        available=None
        for reload_index in range(2):
            for poll in range(13):
                try:
                    baseline=shell_json("ha apps info "+slug+" --no-progress --raw-json",timeout=80)
                except Exception:
                    baseline=None
                info=(baseline or {}).get("data") or {}
                if (baseline and baseline.get("result")=="ok"
                    and info.get("slug")==slug
                    and isinstance(info.get("version_latest"),str)
                    and info["version_latest"].strip()
                    and info.get("repository") in ("core",None)
                    and info.get("state")=="unknown"):
                    available=info
                    break
                if poll<12:_sleep(2)
            if available is not None:break
            if reload_index==0:
                try:
                    shell_json("ha store reload --no-progress --raw-json",timeout=150)
                except Exception:
                    # A timed-out POST may have completed; observe the state,
                    # never assume the POST reply guarantees Store readiness.
                    pass
        assert available is not None,"Fixed official File Editor Store catalogue not ready after bounded discovery"
        assert shell_json("ha supervisor info --no-progress --raw-json",timeout=90).get("result")=="ok"
        assert shell_json("ha core info --no-progress --raw-json",timeout=90).get("result")=="ok"
        # The official CLI may exit nonzero when the Supervisor job fails.
        # Observe only bounded, redacted diagnostics, never raw guest logs.
        stdout, stderr, exit_code = shell.run(
            "ha apps install "+slug+" --no-progress --raw-json", timeout=450
        )
        try:
            import json as _json
            installed=_json.loads("\n".join(stdout))
        except (ValueError, TypeError):
            installed={}
        if exit_code != 0 or installed.get("result") != "ok":
            try:
                current=shell_json("ha apps info "+slug+" --no-progress --raw-json",timeout=90)
                observed=(current.get("data") or {}).get("state","unknown")
                if observed not in ("unknown",None):
                    added=True  # Ensure disposal of a partially installed App.
            except Exception:
                observed="unavailable"
            raw=str(installed.get("message","")) + " " + " ".join(stderr or [])
            category=("outdated_plugins" if "plugin" in raw.lower() and "up to date" in raw.lower()
                      else "network" if "network" in raw.lower() or "pull" in raw.lower()
                      else "permission" if "permission" in raw.lower()
                      else "no_cli_detail")
            raise AssertionError("Official App install refused: category="+category+
                                 " observed_app_state="+str(observed)+
                                 " exit_code="+str(exit_code))
        if installed.get("result")!="ok":
            # Safe disposable environment error code only, never raw tokens.
            message=str(installed.get("message","unknown"))
            category=("not_found" if "not found" in message.lower()
                      else "network" if "network" in message.lower() or "pull" in message.lower()
                      else "permission" if "permission" in message.lower()
                      else "other")
            diagnostic=re.sub(r"[a-zA-Z0-9_-]{32,}", "[opaque]",
                              message.replace("\\n"," "))[:190]
            diagnostic=re.sub(r"[^a-zA-Z0-9 _.,:/()+=-]", "_",diagnostic)
            raise AssertionError("Official File Editor installation denied category="
                                 +category+" detail="+diagnostic)
        added=True
        data=shell_json("ha apps info "+slug+" --no-progress --raw-json",timeout=100).get("data") or {}
        assert data.get("slug")==slug
        assert data.get("state") in ("started","stopped")
        assert isinstance(data.get("version"),str) and data["version"]
        # Native Supervisor version-specific App data roots; no mkdir fallback.
        roots=("/mnt/data/supervisor/apps/data/"+slug,
               "/mnt/data/supervisor/addons/data/"+slug)
        matched=[root for root in roots if shell.run_check(
            "test -d "+root+" && echo PRESENT || echo ABSENT",
            timeout=30)[0].strip()=="PRESENT"]
        assert len(matched)==1 and matched[0] in roots,matched
        path=matched[0]+"/"+marker_name
        # Known in-repository static synthetic bytes: no host/user data.
        import base64
        b64=base64.b64encode(marker_value).decode("ascii")
        assert len(b64)<120
        shell.run_check("printf %s "+b64+" | base64 -d > "+path,timeout=30)
        assert shell.run_check("sha256sum "+path,timeout=30)[0].split()[0]==marker_hash
        created=shell_json('ha backups new --name "P8CU Public App Only Lab" --app '
                           +slug+" --no-progress --raw-json",timeout=460)
        backup_slug=(created.get("data") or {}).get("slug")
        assert created.get("result")=="ok" and re.fullmatch(r"[0-9a-f]{8,16}",backup_slug or "")
        details=shell_json("ha backups info "+backup_slug+" --no-progress --raw-json",timeout=95)
        meta=details.get("data") or {}
        assert details.get("result")=="ok"
        assert meta.get("type")=="partial" and meta.get("protected") is False
        advertised=meta.get("addons")
        assert isinstance(advertised,list)
        assert [x.get("slug") if isinstance(x,dict) else x for x in advertised]==[slug]
        backup="/mnt/data/supervisor/backup/"+backup_slug+".tar"
        assert shell.run_check("test -s "+backup+" && echo PRESENT")[0].strip()=="PRESENT"
        digest=shell.run_check("sha256sum "+backup,timeout=45)[0].split()[0]
        assert re.fullmatch(r"[0-9a-f]{64}",digest)
        boot=shell.run_check("cat /proc/sys/kernel/random/boot_id")[0].strip()
        assert re.fullmatch(r"[0-9a-f-]{36}",boot)
        core=shell_json("ha core info --no-progress --raw-json",timeout=95)
        version=(core.get("data") or {}).get("version")
        assert core.get("result")=="ok" and re.fullmatch(r"[0-9A-Za-z_.+-]{1,64}",version or "")
        transfer=shell.run_check(
            "curl --fail --silent --show-error --max-time 460 "+
            "-H 'X-Hars-Slug: "+backup_slug+"' "+
            "-H 'X-Hars-Boot: "+boot+"' "+
            "-H 'X-Hars-Core: "+version+"' -T "+backup+
            " http://10.0.2.2:18674/archive && echo P8CU_PRIVATE_EXPORT_DONE",
            timeout=480)
        assert "P8CU_PRIVATE_EXPORT_DONE" in transfer
        # Host-side verifier independently checks the EXACT marker bytes from
        # the real official nested backup part, never runs an App operation.
        print("P8CU_GENUINE_HAOS_FILE_EDITOR_APP_ONLY_PARTIAL_BACKUP_EXPORTED_PASS")
    finally:
        if backup_slug and re.fullmatch(r"[0-9a-f]{8,16}",backup_slug):
            removed=shell_json("ha backups remove "+backup_slug+
                               " --no-progress --raw-json",timeout=180)
            assert removed.get("result")=="ok","Disposable archive cleanup failed"
        if added:
            removed=shell_json("ha apps uninstall "+slug+
                               " --no-progress --raw-json",timeout=260)
            assert removed.get("result")=="ok","Disposable official File Editor cleanup failed"
