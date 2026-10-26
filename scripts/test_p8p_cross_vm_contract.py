#!/usr/bin/env python3
"""Defend the cross-VM proof against synthetic, missing, tampered or skipped results."""
import importlib.util
import io
import json
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
from urllib.request import Request,urlopen
from urllib.error import HTTPError,URLError
from xml.etree import ElementTree as ET
root=Path(__file__).resolve().parent.parent
workflow=(root/".github/workflows/p8p-cross-vm-recovery.yml").read_text()
test=(root/"scripts/p8p_cross_vm_backup_pytest.py").read_text()
relay=(root/"scripts/p8p_cross_vm_backup_relay.py").read_text()
assert workflow.count("./run_tests.sh ")==2
assert "-snapshot -accel kvm" in workflow
assert "haos_ova-18.3.qcow2.xz" in workflow
assert "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090" in workflow
assert "90fa213c3441e585d5a60387e9244d0ed38efaf8" in workflow
for marker in ("test_hars_cross_vm_source","test_hars_cross_vm_target","/proc/sys/kernel/random/boot_id","ha backups restore ","sha256sum ","test ! -e ",
               "P8BF_ACTUAL_SECOND_HAOS_AUTOMATION_SOURCE_EXACT_BYTES_RESTORED_PASS",
               "inert_automation()", "expected_automation_sha", "AUTOMATION_PATH"):
    assert marker in test,marker
for safety in ("O_NOFOLLOW","O_EXCL","CAP=1024*1024*1024","os.fsync","os.replace"):
    assert safety in relay,safety
assert "P8BF_DIRECT_HAOS_TO_FRESH_HAOS_AUTOMATION_SOURCE_RESTORED_PASS" in workflow
assert "P8BF_HAOS_THROUGH_RUST_ESCROW_TO_FRESH_HAOS_AUTOMATIONS_RESTORED_PASS" in (root/".github/workflows/p8ab-native-escrow-rollback.yml").read_text()
assert "qm " not in workflow and "pct " not in workflow
spec=importlib.util.spec_from_file_location("junit",root/"scripts/check_p8p_cross_vm_junit.py")
mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod)
with tempfile.TemporaryDirectory() as temp:
    path=Path(temp)/"test.xml"
    names=["test_start_supervisor","test_check_supervisor","test_hars_cross_vm_source"]
    def write(names,extra=None):
        suite=ET.Element("testsuite")
        for name in names:
            child=ET.SubElement(suite,"testcase",{"name":name})
            if name==extra:ET.SubElement(child,"skipped")
        ET.ElementTree(suite).write(path)
    write(names)
    assert "3_OF_3_PASS" in mod.verify(path,"source")
    write(names,names[-1])
    try:mod.verify(path,"source")
    except ValueError:pass
    else:raise AssertionError("skipped QEMU guest accepted")
    write(names[:-1])
    try:mod.verify(path,"source")
    except ValueError:pass
    else:raise AssertionError("absent QEMU guest accepted")
    store=Path(temp)/"relay"
    with socket.socket() as sock:
        sock.bind(("127.0.0.1",0))
        port=sock.getsockname()[1]
    process=subprocess.Popen([sys.executable,str(root/"scripts/p8p_cross_vm_backup_relay.py"),
        "--output-dir",str(store),"--port",str(port)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    try:
        url=f"http://127.0.0.1:{port}"
        for _ in range(50):
            try:urlopen(url+"/manifest",timeout=0.2)
            except HTTPError as ex:
                if ex.code==404:break
            except (URLError,ConnectionRefusedError):pass
            time.sleep(0.05)
        else:raise AssertionError("relay not listening")
        payload=b"not-private-synthetic-HAOS-backup"*180
        hdr={"X-Hars-Slug":"deadbeef","X-Hars-Boot":"12345678-1234-1234-1234-123456789abc","X-Hars-Core":"2026.10.2"}
        request=Request(url+"/archive",data=payload,headers=hdr,method="PUT")
        with urlopen(request,timeout=3) as resp:assert resp.status==201
        with urlopen(url+"/manifest",timeout=3) as resp:manifest=json.load(resp)
        assert manifest["slug"]=="deadbeef"
        assert manifest["size_bytes"]==len(payload)
        import hashlib
        assert manifest["sha256"]==hashlib.sha256(payload).hexdigest()
        with urlopen(url+"/archive",timeout=3) as resp:assert resp.read()==payload
        try:urlopen(request,timeout=3)
        except HTTPError as ex:assert ex.code==409
        else:raise AssertionError("second upload overwrote first")
        assert (store/"archive.tar").read_bytes()==payload
    finally:
        process.terminate()
        process.wait(timeout=5)
print("PASS: two separate pinned QEMU boots required; strict JUnit; one-use nofollow relay with SHA256 and overwrite denial")
