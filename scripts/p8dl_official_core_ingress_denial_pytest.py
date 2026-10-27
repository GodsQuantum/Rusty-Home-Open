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
def test_hars_official_file_editor_ingress_unauthenticated_denial(shell,shell_json):
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
        # P8dj: the app MUST run under the REAL official Supervisor,
        # not a fabricated token/bridge: it fetches its config through
        # the actual Supervisor API. Ingress itself remains Core-protected.
        current=shell_json("ha apps info "+slug+" --no-progress --raw-json",timeout=90)
        if (current.get("data") or {}).get("state")!="started":
            started=shell_json("ha apps start "+slug+" --no-progress --raw-json",timeout=250)
            assert started.get("result")=="ok", "Official Supervisor refused to start File Editor"
        for check in range(45):
            status=shell_json("ha apps info "+slug+" --no-progress --raw-json",timeout=75)
            if (status.get("data") or {}).get("state")=="started":break
            if check<44:_sleep(2)
        else:
            raise AssertionError("Real official File Editor never reached started state")
        print("P8DJ_REAL_HAOS_SUPERVISOR_LAUNCHED_OFFICIAL_EDITOR_APP_PASS")
        # A request ORIGINATING INSIDE the actual Supervisor container has
        # trusted Supervisor IP (172.30.32.2), as official Ingress expects.
        # This is a genuine backend HTTP test, NOT an authenticated user
        # session; never send browser traffic or expose port 8099.
        import shlex as _shlex
        probe=(
          'import urllib.request as u; '
          'r=u.urlopen("http://core-configurator:8099/",timeout=5); '
          'b=r.read(262144).lower(); '
          'assert r.status==200 and b"<html" in b and b"configurator" in b; '
          'print("P8DJ_REAL_OFFICIAL_SUPERVISOR_TRUSTED_INTERNAL_HTTP_200_HTML_PASS")'
        )
        command=('for i in $(seq 1 40); do docker exec hassio_supervisor python3 -c '
                 +_shlex.quote(probe)+' 2>/dev/null && break; sleep 2; done')
        observed=shell.run_check(command,timeout=220)
        assert "P8DJ_REAL_OFFICIAL_SUPERVISOR_TRUSTED_INTERNAL_HTTP_200_HTML_PASS" in observed, (
          "Real upstream Supervisor-managed File Editor did not answer HTML 200")
        # The verified real App is operating behind its official trusted
        # Supervisor backend. Probe the actual Core ingress proxy WITHOUT
        # ever creating credentials, tokens or sessions. Forged user headers
        # must not make an unauthenticated request an authorized login.
        after_info=shell_json("ha apps info "+slug+" --no-progress --raw-json",timeout=90)
        model=after_info.get("data") or {}
        ingress=model.get("ingress_url")
        assert isinstance(ingress,str), "Official App ingress URL unavailable"
        from urllib.parse import urlsplit as _split
        path=_split(ingress).path
        assert path.startswith("/api/hassio_ingress/"), "Official App Core ingress path absent"
        assert re.fullmatch(r"/api/hassio_ingress/[A-Za-z0-9_./-]{8,240}",path)
        # Core proxy itself forwards the request but Supervisor must reject
        # because there is no ingress_session cookie.
        p8dn_name="rusty_home_p8dn_http_write_synthetic.txt"
        p8dn_value=b"P8DN_OFFICIAL_FILE_EDITOR_REAL_HTTP_WRITE_20261027\n"
        p8dn_hash=hashlib.sha256(p8dn_value).hexdigest()
        # File Editor enforces /homeassistant as its official writable BASEPATH.
        # This is a disposable guest Core config, never the real VM100.
        p8dn_host="/mnt/data/supervisor/homeassistant/"+p8dn_name
        assert shell.run_check("test -d /mnt/data/supervisor/homeassistant && echo PRESENT",
                               timeout=30)[0].strip()=="PRESENT"
        assert shell.run_check("test ! -e "+p8dn_host+" && echo ABSENT",timeout=30)[0].strip()=="ABSENT"
        script='''import urllib.request as u,urllib.error as e,urllib.parse as p
import json,sys
path,mode=sys.argv[1:3]
name="rusty_home_p8dn_http_write_synthetic.txt"
value="P8DN_OFFICIAL_FILE_EDITOR_REAL_HTTP_WRITE_20261027\\n"
form=p.urlencode({"filename":"/homeassistant/"+name,"text":value}).encode()
headers={"Content-Type":"application/x-www-form-urlencoded"}
if mode=="deny":
    checks=({"X-Remote-User-Id":"forged-ci","X-Remote-User-Groups":"administrators"},
            {},{"Cookie":"ingress_session=invalid-lab-session-value"})
    for h in checks:
        try:
            with u.urlopen(u.Request("http://homeassistant:8123"+path,headers=h),timeout=15) as res:
                code=res.status
                res.read(1024)
        except e.HTTPError as err:
            code=err.code
        assert code in (401,403), "Untrusted Core ingress GET was not denied"
    print("P8DL_REAL_CORE_SUPERVISOR_INGRESS_MISSING_FORGED_AND_SPOOFED_HEADER_DENIED_PASS")
    try:
        with u.urlopen(u.Request("http://homeassistant:8123"+path.rstrip("/")+"/api/save",
                     data=form,headers=headers),timeout=15) as res:
            code=res.status
            res.read(1024)
    except e.HTTPError as err:
        code=err.code
    # Core may redirect to its login UI or reject the method before auth.
    # Only the independent guest filesystem absence below proves NO WRITE.
    assert code in (301,302,303,307,308,401,403,404,405), (
        "Untrusted Core ingress POST was unexpectedly accepted or crashed")
    print("P8DN_CORE_INGRESS_UNAUTHENTICATED_POST_HTTP_STATUS_"+str(code))
elif mode=="write":
    with u.urlopen(u.Request("http://core-configurator:8099/api/save",
                   data=form,headers=headers),timeout=15) as res:
        assert res.status==200
        body=json.loads(res.read(8192))
    assert body.get("error") is False, "Official File Editor refused genuine HTTP save"
    print("P8DN_REAL_OFFICIAL_EDITOR_HTTP_POST_SAVED_SYNTHETIC_FILE_PASS")
else:
    raise AssertionError("Unknown fixture mode")
'''
        import base64 as _base64
        import shlex as _shlex
        encoded=_base64.b64encode(script.encode("utf-8")).decode("ascii")
        guest_tmp="/tmp/rh-p8dl-official-ingress.b64"
        guest_python="/tmp/rh-p8dl-official-ingress.py"
        guest_container="/tmp/rh-p8dl-inspect.py"
        try:
            shell.run_check(": > "+guest_tmp,timeout=20)
            # Limit every QEMU serial line to ~128 chars; previous real HAOS
            # tests prove longer serial writes may truncate unpredictably.
            for offset in range(0,len(encoded),72):
                shell.run_check("printf %s "+encoded[offset:offset+72]+
                                " >> "+guest_tmp,timeout=20)
            shell.run_check("base64 -d "+guest_tmp+" > "+guest_python,timeout=30)
            shell.run_check("docker cp "+guest_python+" hassio_supervisor:"+
                            guest_container,timeout=50)
            outcome=shell.run_check("docker exec hassio_supervisor python3 "+
                                    guest_container+" "+_shlex.quote(path)+" deny",timeout=90)
            assert "P8DL_REAL_CORE_SUPERVISOR_INGRESS_MISSING_FORGED_AND_SPOOFED_HEADER_DENIED_PASS" in outcome
            assert any(line.startswith("P8DN_CORE_INGRESS_UNAUTHENTICATED_POST_HTTP_STATUS_")
                       for line in outcome), "Core ingress POST status not observed"
            assert shell.run_check("test ! -e "+p8dn_host+" && echo ABSENT",timeout=30)[0].strip()=="ABSENT", (
                "Unauthenticated Core ingress POST created a guest Core config file")
            print("P8DN_REAL_CORE_INGRESS_UNAUTHENTICATED_POST_NO_GUEST_FILE_PASS")
            positive=shell.run_check("docker exec hassio_supervisor python3 "+
                                     guest_container+" "+_shlex.quote(path)+" write",timeout=90)
            assert "P8DN_REAL_OFFICIAL_EDITOR_HTTP_POST_SAVED_SYNTHETIC_FILE_PASS" in positive
            assert shell.run_check("sha256sum "+p8dn_host,timeout=30)[0].split()[0]==p8dn_hash
            print("P8DN_REAL_OFFICIAL_EDITOR_HTTP_WRITE_GUEST_SHA256_PASS")
            stopped=shell_json("ha apps stop "+slug+" --no-progress --raw-json",timeout=200)
            assert stopped.get("result")=="ok", "Official Supervisor App stop failed"
            restarted=shell_json("ha apps start "+slug+" --no-progress --raw-json",timeout=250)
            assert restarted.get("result")=="ok", "Official Supervisor App restart failed"
            for attempt in range(45):
                status=shell_json("ha apps info "+slug+" --no-progress --raw-json",timeout=75)
                if (status.get("data") or {}).get("state")=="started":
                    break
                if attempt<44:_sleep(2)
            else:
                raise AssertionError("Official File Editor did not restart")
            assert shell.run_check("sha256sum "+p8dn_host,timeout=30)[0].split()[0]==p8dn_hash
            post_restart=shell.run_check(command,timeout=220)
            assert "P8DJ_REAL_OFFICIAL_SUPERVISOR_TRUSTED_INTERNAL_HTTP_200_HTML_PASS" in post_restart
            print("P8DN_REAL_OFFICIAL_EDITOR_HTTP_WRITTEN_DATA_SURVIVES_SUPERVISOR_RESTART_PASS")
        finally:
            # Disposable VM: remove only our synthetic probe, never real files.
            shell.run_check("docker exec hassio_supervisor rm -f "+
                            guest_container+"; rm -f "+guest_tmp+" "+guest_python,
                            timeout=30)
            shell.run_check("rm -f "+p8dn_host,timeout=30)
            assert shell.run_check("test ! -e "+p8dn_host+" && echo ABSENT",
                                   timeout=30)[0].strip()=="ABSENT"
        print("P8DL_OFFICIAL_FILE_EDITOR_CORE_INGRESS_FAIL_CLOSED_WITHOUT_BROWSER_SESSION_PASS")
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
