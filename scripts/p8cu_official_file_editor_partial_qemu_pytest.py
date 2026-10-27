"""Official disposable HAOS18.3 real core_configurator PARTIAL App backup.

App is installed through genuine Supervisor and synthetic nonsecret marker
written only to disposable official App data before backup. Never VM100.
"""
import hashlib
import os
from pathlib import Path
import pytest
import re

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
        baseline=shell_json("ha apps info "+slug+" --no-progress --raw-json",timeout=80)
        assert baseline.get("result")=="ok"
        assert (baseline.get("data") or {}).get("state")=="unknown", "Guest contaminated with existing App"
        installed=shell_json("ha apps install "+slug+" --no-progress --raw-json",timeout=450)
        assert installed.get("result")=="ok","Official File Editor installation failed"
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
