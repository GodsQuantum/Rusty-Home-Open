#!/usr/bin/env python3
"""P8cy genuine HAOS18.3 official App-only backup -> INACTIVE private native
filesystem, failpoint rollback, exact fixture byte comparison, no App activation.
"""
import argparse,ast,hashlib,json,os,subprocess,tempfile,stat
from pathlib import Path
from inspect_ha_backup import inspect_backup
p=argparse.ArgumentParser()
p.add_argument("--archive",type=Path,required=True)
p.add_argument("--manifest",type=Path,required=True)
p.add_argument("--native",type=Path,required=True)
args=p.parse_args()
archive=args.archive.resolve(strict=True)
native=args.native.resolve(strict=True)
meta=json.loads(args.manifest.read_text())
with archive.open("rb") as fh:
    sha=hashlib.file_digest(fh,"sha256").hexdigest()
assert sha==meta["sha256"] and archive.stat().st_size==meta["size_bytes"]
info=inspect_backup(archive,{"core_configurator"})
assert info["app_only_partial_archive_identified"] and info["blocking_issues"]==[]
# Load the exact public, fixed inert laboratory fixture constant, never
# private App values, passwords, tokens, user-supplied credentials or a shell.
source=Path(__file__).resolve().parent/"test_p8cu_genuine_file_editor_app_only_archive.py"
marker=None
for node in ast.parse(source.read_text()).body:
    if isinstance(node,ast.Assign) and any(
        isinstance(t,ast.Name) and t.id=="marker" for t in node.targets
    ):
        marker=ast.literal_eval(node.value)
assert isinstance(marker,bytes) and marker.startswith(b"P8CU_") and len(marker)<256
with tempfile.TemporaryDirectory(prefix="p8cy-real-app-") as tmp:
    root=Path(tmp)
    escrow=root/"escrow"
    target=root/"native-inactive"
    escrow.mkdir(mode=0o700)
    target.mkdir(mode=0o700)
    def run(action,*tokens,ok=True,fail=False):
        env=dict(os.environ)
        if fail: env["HARS_P8CY_FAIL_BEFORE_COMMIT"]="1"
        result=subprocess.run([str(native),"--lab-readonly",str(escrow),action,
                                *map(str,tokens)],capture_output=True,text=True,
                                timeout=120,env=env)
        if not ok:
            assert result.returncode != 0,(action,result.stdout)
            assert not result.stdout and "HA_REGISTRY_STAGING_FAIL_CLOSED" in result.stderr
            return
        assert result.returncode==0,(action,result.stderr[-450:])
        value=json.loads(result.stdout)
        assert value["production_ready"] is False
        assert marker.decode(errors="ignore") not in result.stdout
        return value
    stored=run("app-source-ingest",archive)
    assert stored["sha256"]==sha
    assert run("app-source-verify",sha)["archive_sha256"]==sha
    expected=target/("app-core_configurator-"+sha)
    run("app-source-stage-inactive",sha,"core_configurator",target,ok=False,fail=True)
    assert not list(target.iterdir()),"failed App import leaked partial state"
    assert run("app-source-verify",sha)["archive_sha256"]==sha
    print("P8CY_REAL_SUPERVISOR_NATIVE_APP_DATA_INJECTED_FAULT_ATOMIC_ZERO_LEAK_PASS")
    stage=run("app-source-stage-inactive",sha,"core_configurator",target)
    assert stage["action"]=="stage_inactive_native_app_files"
    assert stage["sha256"]==sha and stage["app_count"]==1 and stage["files"]>=1
    assert stage["bytes"]>=len(marker)
    assert stage["app_image_installed"] is False
    assert stage["app_runtime_started"] is False
    assert stage["automation_executed"] is False
    assert expected.is_dir()
    markers=list(expected.rglob("rusty_home_p8cu_nonsecret_synthetic_marker.txt"))
    assert len(markers)==1, "native stage missing or duplicating exact source marker"
    assert markers[0].is_file() and markers[0].read_bytes()==marker
    for member in expected.rglob("*"):
        assert not member.is_symlink()
        assert (member.stat().st_mode & 0o077)==0,"native staged files not private"
        if member.is_file():
            assert member.stat().st_mode & 0o777==0o600
        if member.is_dir():
            assert member.stat().st_mode & 0o777==0o700
    run("app-source-stage-inactive",sha,"core_configurator",target,ok=False)
    assert markers[0].read_bytes()==marker,"exclusive publish changed previously restored data"
    print("P8CY_REAL_OFFICIAL_FILE_EDITOR_APP_BYTES_INACTIVE_PRIVATE_NATIVE_STAGE_PASS")
    saved=escrow/(sha+".tar")
    with saved.open("r+b") as stream:
        stream.seek(-1024,os.SEEK_END)
        old=stream.read(1)
        stream.seek(-1024,os.SEEK_END)
        stream.write(bytes((old[0]^1,)))
        stream.flush();os.fsync(stream.fileno())
    fresh=root/"second-inactive-restore"
    fresh.mkdir(mode=0o700)
    run("app-source-stage-inactive",sha,"core_configurator",fresh,ok=False)
    assert not list(fresh.iterdir()),"bad escrow leaked native App files"
    run("app-source-verify",sha,ok=False)
    assert markers[0].read_bytes()==marker
    with archive.open("rb") as stream:
        assert hashlib.file_digest(stream,"sha256").hexdigest()==sha
    print("P8CY_REAL_SUPERVISOR_PRIVATE_NATIVE_FILES_PERSIST_AFTER_ESCROW_TAMPER_PASS")
    print("P8CY_REAL_APP_DATA_STAGED_ONLY_NOT_EXECUTED_OR_CONNECTED_PASS")
