#!/usr/bin/env python3
"""Genuine official Mosquitto Supervisor backup -> native Rust private data.

Actual public HAOS 18.3 second App, not a File Editor duplicate. Staging is
INACTIVE and no MQTT listener, user identity, radio or household data exists.
"""
import argparse,ast,hashlib,json,os,subprocess,tempfile
from pathlib import Path
from inspect_ha_backup import inspect_backup

parser=argparse.ArgumentParser()
parser.add_argument("--archive",type=Path,required=True)
parser.add_argument("--manifest",type=Path,required=True)
parser.add_argument("--native",type=Path,required=True)
args=parser.parse_args()
archive=args.archive.resolve(strict=True)
native=args.native.resolve(strict=True)
meta=json.loads(args.manifest.read_text())
assert archive.stat().st_size==meta["size_bytes"]
with archive.open("rb") as h:
    digest=hashlib.file_digest(h,"sha256").hexdigest()
assert digest==meta["sha256"]

inspected=inspect_backup(archive,{"core_mosquitto"})
assert inspected["app_only_partial_archive_identified"] is True
assert inspected["blocking_issues"]==[]
assert inspected["addons_advertised_count"]==1
assert inspected["addon_payloads_identified_count"]==1
assert inspected["can_restore"] is False
assert inspected["eligible_for_migration"] is False

original=Path(__file__).with_name("p8do_official_mosquitto_qemu_pytest.py")
marker=None
marker_filename="rusty_home_p8do_nonsecret_mqtt_marker.txt"
for node in ast.parse(original.read_text()).body:
    if isinstance(node,ast.FunctionDef) and node.name=="test_hars_official_mosquitto_app_only_partial_export":
        for inside in node.body:
            if isinstance(inside,ast.Assign) and any(isinstance(t,ast.Name) and t.id=="marker_value" for t in inside.targets):
                marker=ast.literal_eval(inside.value)
assert isinstance(marker,bytes) and marker.startswith(b"P8DO_REAL") and len(marker)<256
with tempfile.TemporaryDirectory(prefix="p8do-real-mosquitto-app-") as tmp:
    workspace=Path(tmp)
    escrow=workspace/"escrow";escrow.mkdir(mode=0o700)
    target=workspace/"inactive";target.mkdir(mode=0o700)
    def native_cmd(*params,positive=True):
        completed=subprocess.run([str(native),"--lab-readonly",str(escrow),*map(str,params)],
            text=True,capture_output=True,timeout=125)
        if not positive:
            assert completed.returncode!=0
            assert "HA_REGISTRY_STAGING_FAIL_CLOSED" in completed.stderr
            assert not completed.stdout
            return None
        assert completed.returncode==0,completed.stderr[-380:]
        outcome=json.loads(completed.stdout)
        assert outcome["production_ready"] is False
        assert marker.decode(errors="ignore") not in completed.stdout
        return outcome
    receipt=native_cmd("app-source-ingest",archive)
    assert receipt["sha256"]==digest and receipt["app_count"]==1
    assert native_cmd("app-source-verify",digest)["archive_sha256"]==digest
    assert native_cmd("app-source-ingest",archive)["sha256"]==digest
    assert len(list(escrow.glob("*.tar")))==1
    print("P8DO_REAL_HAOS_SUPERVISOR_MOSQUITTO_PARTIAL_APP_SHA_ESCROW_PASS")
    fail_env=dict(os.environ,HARS_P8CY_FAIL_BEFORE_COMMIT="1")
    crashed=subprocess.run([str(native),"--lab-readonly",str(escrow),
                            "app-source-stage-inactive",digest,"core_mosquitto",str(target)],
                           capture_output=True,text=True,timeout=125,env=fail_env)
    assert crashed.returncode!=0 and not crashed.stdout
    assert not list(target.iterdir())
    print("P8DO_REAL_MOSQUITTO_APP_PRIVATE_STAGE_FAILPOINT_ZERO_PUBLICATION_PASS")
    restored=native_cmd("app-source-stage-inactive",digest,"core_mosquitto",target)
    assert restored["files"]>=1 and restored["app_image_installed"] is False
    assert restored["app_runtime_started"] is False
    assert restored["automation_executed"] is False
    dest=target/("app-core_mosquitto-"+digest)
    assert dest.is_dir() and dest.stat().st_mode&0o077==0
    matches=list(dest.rglob(marker_filename))
    assert len(matches)==1 and matches[0].is_file() and matches[0].read_bytes()==marker
    assert all(not item.is_symlink() and item.stat().st_mode&0o077==0 for item in dest.rglob("*"))
    native_cmd("app-source-stage-inactive",digest,"core_mosquitto",target,positive=False)
    assert matches[0].read_bytes()==marker
    print("P8DO_REAL_OFFICIAL_SECOND_APP_MOSQUITTO_NATIVE_PRIVATE_FILES_EXACT_BYTES_PASS")
    copied=escrow/(digest+".tar")
    with copied.open("r+b") as handle:
        handle.seek(-1024,os.SEEK_END)
        old=handle.read(1)
        handle.seek(-1024,os.SEEK_END)
        handle.write(bytes([old[0]^1]))
        handle.flush()
        os.fsync(handle.fileno())
    second=workspace/"second";second.mkdir(mode=0o700)
    native_cmd("app-source-stage-inactive",digest,"core_mosquitto",second,positive=False)
    assert not list(second.iterdir())
    assert matches[0].read_bytes()==marker
    with archive.open("rb") as h:
        assert hashlib.file_digest(h,"sha256").hexdigest()==digest
    print("P8DO_REAL_MOSQUITTO_CORRUPT_ESCROW_REFUSED_ORIGINAL_AND_STAGED_DATA_PRESERVED_PASS")
