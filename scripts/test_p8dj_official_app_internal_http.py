#!/usr/bin/env python3
"""P8dj: genuine HAOS App bytes to official File Editor internal HTTP service
-> genuine official OCI image on THROWAWAY rootless isolated GH runner.

No HAOS host, public network, incoming port mapping or Supervisor secrets.
Not equivalent to ingress/auth/Core parity or full App lifecycle certification.
"""
import argparse,ast,hashlib,json,os,re,subprocess,tempfile,time
from pathlib import Path
from inspect_ha_backup import inspect_backup
from test_p8di_official_app_crash_restart import verify_restart
P=argparse.ArgumentParser()
P.add_argument("--archive",type=Path,required=True)
P.add_argument("--manifest",type=Path,required=True)
P.add_argument("--native",type=Path,required=True)
P.add_argument("--image",default="ghcr.io/home-assistant/app-configurator:6.3.0")
a=P.parse_args()
assert os.environ.get("GITHUB_ACTIONS")=="true" and os.environ.get("RUNNER_TEMP")
assert os.geteuid()!=0
assert a.image=="ghcr.io/home-assistant/app-configurator:6.3.0"
assert subprocess.run(["podman","info","--format","{{.Host.Security.Rootless}}"],capture_output=True,text=True,check=True).stdout.strip()=="true"
archive=a.archive.resolve(strict=True)
native=a.native.resolve(strict=True)
manifest=json.loads(a.manifest.read_text())
with archive.open("rb") as stream:
    sha=hashlib.file_digest(stream,"sha256").hexdigest()
assert sha==manifest["sha256"] and archive.stat().st_size==manifest["size_bytes"]
result=inspect_backup(archive,{"core_configurator"})
assert result["app_only_partial_archive_identified"] and not result["blocking_issues"]
source=Path(__file__).with_name("test_p8cu_genuine_file_editor_app_only_archive.py")
marker=None
for node in ast.parse(source.read_text()).body:
    if isinstance(node,ast.Assign) and any(isinstance(t,ast.Name) and t.id=="marker" for t in node.targets):
        marker=ast.literal_eval(node.value)
assert marker and marker.startswith(b"P8CU_") and len(marker)<256

def run(command,*,expect=True,timeout=90):
    result=subprocess.run(command,capture_output=True,text=True,timeout=timeout)
    if expect:
        assert result.returncode==0,(command[0:2],result.stderr[-800:])
    return result

# Actual upstream GHCR official version tag is resolved to one OCI image ID
# and used unchanged throughout this single disposable test.
run(["podman","pull","--quiet",a.image],timeout=400)
identity=run(["podman","image","inspect",a.image,"--format","{{.Id}}"]).stdout.strip()
# Podman versions differ: .Id can be 64 hex chars, or sha256:64 hex.
# Normalize safely; NEVER fall back to a mutable tag for execution.
assert re.fullmatch(r"(?:sha256:)?[0-9a-f]{64}",identity),(
    "Unexpected non-digest Podman image ID format "+str(len(identity)))
identity="sha256:"+identity.removeprefix("sha256:")
assert run(["podman","image","inspect",a.image,"--format","{{.Architecture}}"]).stdout.strip()=="amd64"
print("P8DF_GENUINE_PUBLIC_OFFICIAL_APP_CONFIGURATOR_PINNED_SESSION_IMAGE_PASS",flush=True)
with tempfile.TemporaryDirectory(prefix="p8df-real-app-runtime-",dir=os.environ["RUNNER_TEMP"]) as tmp:
    root=Path(tmp)
    escrow=root/"escrow";escrow.mkdir(mode=0o700)
    stage_root=root/"stage";stage_root.mkdir(mode=0o700)
    def native_cmd(*tokens):
        call=run([str(native),"--lab-readonly",str(escrow),*map(str,tokens)],timeout=120)
        obj=json.loads(call.stdout)
        assert obj["production_ready"] is False
        return obj
    admission=native_cmd("app-source-ingest",archive)
    assert admission["sha256"]==sha
    assert native_cmd("app-source-verify",sha)["archive_sha256"]==sha
    stage=native_cmd("app-source-stage-inactive",sha,"core_configurator",stage_root)
    assert stage["files"]>=1 and not stage["app_runtime_started"]
    restored=stage_root/("app-core_configurator-"+sha)
    assert restored.is_dir() and restored.stat().st_mode&0o077==0
    markers=list(restored.rglob("rusty_home_p8cu_nonsecret_synthetic_marker.txt"))
    assert len(markers)==1 and markers[0].read_bytes()==marker
    # The internal app data directory created by official nested App tar is
    # actually "data/"; ensure we never bind a random unverified /config.
    data_dir=restored/"data"
    assert data_dir.is_dir()
    assert markers[0].parent==data_dir
    assert data_dir.stat().st_mode&0o077==0
    print("P8DF_REAL_HAOS_SUPERVISOR_APP_DATA_NATIVE_RUST_PRIVATE_RESTAGE_PASS",flush=True)
    name="p8df-file-editor-unique-offline-ci"
    try:
        # Image's s6 process startup may require internal root; this is
        # ROOTLESS PODMAN, with no NET, host ports, host devices, host API or
        # Supervisor token; never grant privileged/host networking.
        launch=[
            "podman","run","--detach","--replace","--name",name,
            "--network","none","--read-only","--cap-drop","ALL",
            "--security-opt","no-new-privileges","--pids-limit","128",
            "--memory","512m","--tmpfs","/run:rw,size=64m,mode=1777",
            "--tmpfs","/tmp:rw,size=64m,mode=1777",
            "--tmpfs","/var/log:rw,size=16m,mode=0755",
            "--mount",f"type=bind,source={data_dir},destination=/data,rw=true",
            identity
        ]
        container=run(launch,expect=False,timeout=75)
        if container.returncode!=0:
            raise AssertionError("Official App image launch denied: "+container.stderr[-600:])
        alive=False
        for _ in range(40):
            state=run(["podman","inspect",name,"--format","{{.State.Running}}"],expect=False)
            if state.returncode==0 and state.stdout.strip()=="true":
                alive=True
                break
            time.sleep(.3)
        if not alive:
            status=run(["podman","logs",name],expect=False)
            raise AssertionError("Official image did not stay alive: "+status.stderr[-650:])
        assert run(["podman","inspect",name,"--format","{{.HostConfig.NetworkMode}}"]).stdout.strip()=="none"
        assert run(["podman","inspect",name,"--format","{{.HostConfig.Privileged}}"]).stdout.strip()=="false"
        assert run(["podman","inspect",name,"--format","{{.HostConfig.ReadonlyRootfs}}"]).stdout.strip()=="true"
        assert run(["podman","inspect",name,"--format","{{json .HostConfig.PortBindings}}"]).stdout.strip() in ("{}","null")
        mounts=json.loads(run(["podman","inspect",name,"--format","{{json .Mounts}}"]).stdout)
        assert len([m for m in mounts if m.get("Destination")=="/data" and m.get("Source")==str(data_dir)])==1
        print("P8DF_OFFICIAL_HAOS_FILE_EDITOR_CONTAINER_ROOTLESS_NO_NETWORK_START_PASS",flush=True)
        # P8dj NEW: verify real APP HTTP RESPONSE from INSIDE isolated official
        # container namespace, never open a host port or bypass HA Ingress.
        # Image contains the real python web app; urllib is stdlib, not mock.
        # A process alive without a responding HTML editor is NOT sufficient.
        probe_source = """
import sys, time, urllib.request, urllib.error
url="http://127.0.0.1:8099/"
for attempt in range(60):
    try:
        with urllib.request.urlopen(url, timeout=3) as response:
            status=response.status
            ctype=response.headers.get("Content-Type","").lower()
            data=response.read(256*1024)
        if status==200 and ("html" in ctype or b"<html" in data.lower()) and b"configurator" in data.lower():
            print("P8DJ_REAL_UPSTREAM_CONFIGURATOR_HTML_OK")
            sys.exit(0)
        print("P8DJ_HTTP_NOT_READY status="+str(status)+",type="+ctype[:64],file=sys.stderr)
    except Exception as exc:
        print("P8DJ_HTTP_NOT_READY exception="+type(exc).__name__,file=sys.stderr)
    time.sleep(.5)
sys.exit(21)
"""
        probe = run(["podman", "exec", name, "python3", "-B", "-c", probe_source],
                    expect=False,timeout=175)
        if probe.returncode!=0:
            logs=run(["podman","logs",name],expect=False,timeout=25)
            # Never log a token, shell env, App private data or secrets. Print
            # only bounded probe diagnosis and bounded s6 startup tail.
            diagnostic="; ".join(probe.stderr.splitlines()[-3:])[:350]
            safe_tail="; ".join((logs.stdout+logs.stderr).splitlines()[-7:])[:600]
            raise AssertionError("REAL_OFFICIAL_APP_HTTP_UNAVAILABLE: "+diagnostic+"; startup="+safe_tail)
        assert "P8DJ_REAL_UPSTREAM_CONFIGURATOR_HTML_OK" in probe.stdout
        assert run(["podman","inspect",name,"--format","{{.HostConfig.NetworkMode}}"]).stdout.strip()=="none"
        assert run(["podman","port",name]).stdout.strip()==""
        print("P8DJ_REAL_FILE_EDITOR_INTERNAL_HTML_SERVE_NO_HOST_INGRESS_BYPASS_PASS",flush=True)
        probe=run(["podman","exec",name,"sh","-c",
                   "test -s /data/rusty_home_p8cu_nonsecret_synthetic_marker.txt && echo P8DF_VISIBLE"],
                   expect=False,timeout=20)
        assert probe.returncode==0 and "P8DF_VISIBLE" in probe.stdout
        print("P8DF_REAL_RESTORED_FILE_EDITOR_MARKER_VISIBLE_INSIDE_RUNNING_OFFICIAL_IMAGE_PASS",flush=True)
        verify_restart(run,name,launch,data_dir,markers[0],marker,archive,sha,identity)
        # A successful process and private marker is milestone 5 only.
        # Do not fake ingress authentication or imply a functioning UI.
    finally:
        run(["podman","rm","--force",name],expect=False,timeout=50)
    with archive.open("rb") as stream:
        assert hashlib.file_digest(stream,"sha256").hexdigest()==sha
    assert markers[0].read_bytes()==marker
    print("P8DF_REAL_NATIVE_APP_RUNTIME_ISOLATED_CLEANUP_ORIGINAL_PRESERVED_PASS",flush=True)
