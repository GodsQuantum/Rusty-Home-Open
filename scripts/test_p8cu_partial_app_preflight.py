#!/usr/bin/env python3
"""P8cu structural preflight, App-only synthetic archive and adversarial inputs.

No extraction, no decryption, no tokens and no actual App execution.
"""
from pathlib import Path
from inspect_ha_backup import inspect_backup,InvalidBackup
import io,json,tarfile,tempfile,gzip

def archive(path, *, kind="partial", addons=("core_configurator",), protected=False,
            payloads=("core_configurator.tar.gz",), core=False, unsafe=None):
    manifest={"name":"P8CU Synthetic Nonsecret App Data","slug":"123abcde",
              "type":kind,"protected":protected,"addons":list(addons)}
    with tarfile.open(path,"w") as tf:
        def item(name,bytes_,type_=tarfile.REGTYPE):
            info=tarfile.TarInfo(name)
            info.type=type_
            info.size=len(bytes_) if type_==tarfile.REGTYPE else 0
            tf.addfile(info,io.BytesIO(bytes_) if type_==tarfile.REGTYPE else None)
        item("backup.json",json.dumps(manifest).encode())
        for name in payloads:
            payload=b"SecureTar"+b"x"*90 if protected else gzip.compress(b"fictional app only")
            item(name,payload)
        if core: item("homeassistant.tar.gz",gzip.compress(b"fictive registry"))
        if unsafe: item(unsafe,b"malicious")

with tempfile.TemporaryDirectory(prefix="p8cu-synthetic-no-user-data-") as tmp:
    p=Path(tmp)/"backup.tar"
    archive(p)
    yes=inspect_backup(p)
    assert yes["app_only_partial_archive_identified"] is True,yes
    assert yes["addons_advertised_count"]==1
    assert yes["addon_payloads_identified_count"]==1
    assert yes["homeassistant_payload_identified"] is False
    assert yes["blocking_issues"]==[]
    assert yes["can_restore"] is False
    assert yes["eligible_for_migration"] is False
    assert yes["mode"]=="offline_metadata_only_no_decryption_no_extraction"
    print("P8CU_APP_ONLY_PARTIAL_OFFLINE_PREFLIGHT_NO_FALSE_CORE_REQUIREMENT_PASS")
    for key,kwargs in {
      "no_addons":dict(addons=(),payloads=()),
      "missing_addon":dict(payloads=()),
      "protected":dict(protected=True),
      "mislabeled_full":dict(kind="full"),
      "unknown_payload":dict(payloads=("unrelated.tar.gz",)),
    }.items():
        archive(p,**kwargs)
        out=inspect_backup(p)
        assert out["app_only_partial_archive_identified"] is False,(key,out)
        assert out["blocking_issues"],key
    archive(p,core=True)
    assert inspect_backup(p)["app_only_partial_archive_identified"] is False
    assert inspect_backup(p)["blocking_issues"]==[]
    archive(p,unsafe="../escape")
    try: inspect_backup(p)
    except InvalidBackup: pass
    else: raise AssertionError("path traversal accepted")
    archive(p,unsafe="backup.json")
    try: inspect_backup(p)
    except InvalidBackup: pass
    else: raise AssertionError("duplicate manifest accepted")
    print("P8CU_FULL_PARTIAL_MISSING_PROTECTED_MALFORMED_AND_TRAVERSAL_REFUSED_PASS")
