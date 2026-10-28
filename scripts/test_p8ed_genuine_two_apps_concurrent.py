#!/usr/bin/env python3
"""P8ed: actual two-app HAOS private atomic backup -> simultaneous real OCI.

ONLY unprivileged disposable GitHub CI. Real official Core File Editor App
and upstream MQTT broker with entirely FICTIONAL LOCAL credentials. No
authentication delegation, public network, production radios or household.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

from haos_mosquitto_local_auth_lab import check_options, generate, MigrationRefused
from inspect_ha_backup import inspect_backup
from p8du_two_real_apps_qemu_pytest import APP_MARKERS

ap=argparse.ArgumentParser()
ap.add_argument("--editor", type=Path, required=True)
ap.add_argument("--editor-manifest", type=Path, required=True)
ap.add_argument("--mosquitto", type=Path, required=True)
ap.add_argument("--mosquitto-manifest", type=Path, required=True)
ap.add_argument("--native", type=Path, required=True)
a=ap.parse_args()

assert os.environ.get("GITHUB_ACTIONS")=="true" and os.environ.get("RUNNER_TEMP")
assert os.geteuid()!=0
assert subprocess.run(["podman","info","--format","{{.Host.Security.Rootless}}"],
                      check=True,capture_output=True,text=True).stdout.strip()=="true"

def cmd(argv, success=True, timeout=120):
    r=subprocess.run(argv, capture_output=True, text=True, timeout=timeout)
    if success:
        # Never log env / local account or password; discard long external logs.
        assert r.returncode==0,(argv[:2],r.returncode,r.stderr[-350:])
    return r

def sha256(path):
    with path.open("rb") as f:
        return hashlib.file_digest(f,"sha256").hexdigest()

slugs=("core_configurator","core_mosquitto")
archives=(a.editor.resolve(strict=True),a.mosquitto.resolve(strict=True))
meta=(json.loads(a.editor_manifest.read_text()),json.loads(a.mosquitto_manifest.read_text()))
assert meta[0]["source_boot_id"]==meta[1]["source_boot_id"]
assert meta[0]["core_version"]==meta[1]["core_version"]
digests=[]
for slug,src,info in zip(slugs,archives,meta):
    assert info["size_bytes"]==src.stat().st_size
    digest=sha256(src)
    assert digest==info["sha256"]
    inspected=inspect_backup(src,{slug})
    assert inspected["app_only_partial_archive_identified"] is True
    assert inspected["blocking_issues"]==[]
    assert inspected["addons_advertised_count"]==1
    digests.append(digest)
assert digests[0]!=digests[1]
print("P8ED_SAME_GENUINE_HAOS_GUEST_TWO_ORIGINAL_APP_BACKUPS_VERIFIED_PASS",flush=True)

EDITOR_IMAGE="ghcr.io/home-assistant/app-configurator:6.3.0"
BROKER_IMAGE="docker.io/library/eclipse-mosquitto:2.0.22"
images=[]
for image in (EDITOR_IMAGE,BROKER_IMAGE):
    cmd(["podman","pull","--quiet",image],timeout=300)
    value=cmd(["podman","image","inspect",image,"--format","{{.Id}}"]).stdout.strip()
    assert re.fullmatch(r"(?:sha256:)?[a-f0-9]{64}",value)
    assert cmd(["podman","image","inspect",image,"--format","{{.Architecture}}"]).stdout.strip()=="amd64"
    images.append("sha256:"+value.removeprefix("sha256:"))
assert images[0]!=images[1]
print("P8ED_BOTH_DISTINCT_REAL_APP_OCI_IMAGES_IMMUTABLY_BOUND_PASS",flush=True)

# Genuine default official Supervisor Mosquitto has no local logins; MUST
# refuse instead of falling back to anonymous when HA delegation is absent.
try:
    check_options({"logins":[],"require_certificate":False,
                   "customize":{"active":False,"folder":"mosquitto"}})
except MigrationRefused:
    pass
else:
    raise AssertionError("unsafe official default HA delegated login unexpectedly accepted")

with tempfile.TemporaryDirectory(prefix="p8ed-two-genuine-apps-",dir=os.environ["RUNNER_TEMP"]) as temp:
    root=Path(temp)
    escrow=root/"escrow";escrow.mkdir(mode=0o700)
    stage_root=root/"stage";stage_root.mkdir(mode=0o700)
    config=root/"broker-config";config.mkdir(mode=0o700)
    native=a.native.resolve(strict=True)
    def native_call(*tokens):
        result=cmd([str(native),"--lab-readonly",str(escrow),*map(str,tokens)])
        obj=json.loads(result.stdout)
        assert obj["production_ready"] is False
        return obj
    for archive,digest in zip(archives,digests):
        assert native_call("app-source-ingest",archive)["sha256"]==digest
        assert native_call("app-source-verify",digest)["archive_sha256"]==digest
    pair=native_call("app-source-stage-pair-inactive","p8ed-two-real-oci",
                     slugs[0],digests[0],slugs[1],digests[1],stage_root)
    assert pair["app_count"]==2 and pair["atomic_private_pair_published"]
    assert pair["app_runtime_started"] is False and pair["live_data_modified"] is False
    published=stage_root/"pair-p8ed-two-real-oci"
    assert published.is_dir() and len(list(stage_root.iterdir()))==1
    editor_data=published/("app-core_configurator-"+digests[0])/"data"
    mqtt_data=published/("app-core_mosquitto-"+digests[1])/"data"
    markers=[]
    for slug,dir_ in zip(slugs,(editor_data,mqtt_data)):
        filename,data,_port=APP_MARKERS[slug]
        marker=dir_/filename
        assert dir_.is_dir() and dir_.stat().st_mode & 0o077==0
        assert marker.read_bytes()==data
        markers.append(marker)
    print("P8ED_REAL_PAIR_ATOMIC_NATIVE_RUST_STAGE_TWO_PRIVATE_APP_DATASETS_PASS",flush=True)

    # FICTITIOUS LOCAL login: never pass household or Supervisor credentials.
    user="p8ed_lab_operator"
    secret="P8ED_FICTIONAL_CI_ONLY_20261028"
    options={"logins":[{"username":user,"password":secret}],
             "require_certificate":False,"certfile":"fullchain.pem",
             "keyfile":"privkey.pem","customize":{"active":False,"folder":"mosquitto"},
             "log_dest":[],"log_type":[],"debug":False}
    generation=generate(options,config,images[1])
    assert generation["anonymous_allowed"] is False and generation["production_ready"] is False
    conf=(config/"mosquitto.conf").read_text()
    assert "allow_anonymous false" in conf and secret not in conf
    assert secret not in (config/"passwords").read_text()
    print("P8ED_NATIVE_BROKER_LOCAL_AUTH_HASHED_NO_HA_DELEGATION_PASS",flush=True)

    editor="p8ed-real-editor-private"
    mqtt="p8ed-real-mqtt-private"
    uid=f"{os.getuid()}:{os.getgid()}"
    def start_editor():
        cmd(["podman","run","--detach","--replace","--name",editor,
             "--network","none","--read-only","--cap-drop","ALL",
             "--security-opt","no-new-privileges","--pids-limit","128",
             "--memory","512m","--tmpfs","/run:rw,size=64m,mode=1777",
             "--tmpfs","/tmp:rw,size=64m,mode=1777",
             "--tmpfs","/var/log:rw,size=16m,mode=0755",
             "--mount",f"type=bind,source={editor_data},destination=/data,rw=true",
             images[0]],timeout=75)
    def start_mqtt():
        cmd(["podman","run","--detach","--replace","--name",mqtt,
             "--userns=keep-id","--user",uid,"--network","none","--read-only",
             "--cap-drop","ALL","--security-opt","no-new-privileges",
             "--pids-limit","80","--memory","256m",
             "--tmpfs","/tmp:rw,size=32m,mode=1777",
             "--mount",f"type=bind,source={config},destination=/mosquitto/config,readonly=true",
             "--mount",f"type=bind,source={mqtt_data},destination=/mosquitto/data,rw=true",
             images[1]],timeout=75)
    def inspect_live(name,data_mount):
        for n in range(30):
            v=cmd(["podman","inspect",name,"--format","{{.State.Running}}"],success=False)
            if v.returncode==0 and v.stdout.strip()=="true":
                break
            time.sleep(.4)
        else:
            raise AssertionError("app OCI did not remain alive: "+name)
        assert cmd(["podman","inspect",name,"--format","{{.HostConfig.NetworkMode}}"]).stdout.strip()=="none"
        assert cmd(["podman","inspect",name,"--format","{{.HostConfig.Privileged}}"]).stdout.strip()=="false"
        assert cmd(["podman","inspect",name,"--format","{{.HostConfig.ReadonlyRootfs}}"]).stdout.strip()=="true"
        assert cmd(["podman","port",name]).stdout.strip()==""
        mounts=json.loads(cmd(["podman","inspect",name,"--format","{{json .Mounts}}"]).stdout)
        assert len([m for m in mounts if m.get("Source")==str(data_mount)])==1
    def mqtt_client(bin_,*,password=secret,extra=(),success=True):
        args=["podman","exec",mqtt,bin_,"-h","127.0.0.1","-p","1883"]
        if password is not None:
            args+=["-u",user,"-P",password] # fictional local password only
        args+=list(extra)
        return cmd(args,success=success,timeout=12)
    def retained_subscribe():
        return mqtt_client("mosquitto_sub",extra=("-C","1","-W","2","-t",topic),success=False)
    topic="p8ed/real-two-apps/retained"
    payload="P8ED_REAL_SIMULTANEOUS_QOS1_RETAINED"
    editor_saved=editor_data/"p8ed_real_editor_writer.txt"
    try:
        start_editor()
        start_mqtt()
        inspect_live(editor,editor_data)
        inspect_live(mqtt,mqtt_data)
        print("P8ED_TWO_REAL_APP_PROCESSES_SIMULTANEOUS_ROOTLESS_NO_EGRESS_PASS",flush=True)
        for _ in range(36):
            sent=mqtt_client("mosquitto_pub",
              extra=("-q","1","-r","-t",topic,"-m",payload),success=False)
            if sent.returncode==0:
                break
            time.sleep(.4)
        else:
            raise AssertionError("authenticated MQTT never accepted actual publish")
        assert retained_subscribe().stdout.strip()==payload
        assert mqtt_client("mosquitto_sub",password="P8ED_WRONG_CI_ONLY",
                           extra=("-C","1","-W","1","-t",topic),success=False).returncode!=0
        assert mqtt_client("mosquitto_sub",password=None,
                           extra=("-C","1","-W","1","-t",topic),success=False).returncode!=0
        write=cmd(["podman","exec",editor,"sh","-c",
                   "printf 'P8ED_EDITOR_WRITTEN_IN_CONTAINER\n' > /data/p8ed_real_editor_writer.txt"],timeout=22)
        assert write.returncode==0
        assert editor_saved.read_text()=="P8ED_EDITOR_WRITTEN_IN_CONTAINER\n"
        assert markers[0].read_bytes()==APP_MARKERS[slugs[0]][1]
        assert markers[1].read_bytes()==APP_MARKERS[slugs[1]][1]
        print("P8ED_CONCURRENT_EDITOR_REAL_WRITE_MQTT_AUTH_QOS1_GOOD_BAD_ANON_DENIED_PASS",flush=True)

        # Kill exactly File Editor, prove broker still serves original retained
        # message while one of the TWO actual App OCI processes is absent.
        cmd(["podman","kill","--signal","SIGKILL",editor],timeout=25)
        assert cmd(["podman","inspect",editor,"--format","{{.State.Running}}"]).stdout.strip()=="false"
        inspect_live(mqtt,mqtt_data)
        assert retained_subscribe().stdout.strip()==payload
        assert editor_saved.read_text()=="P8ED_EDITOR_WRITTEN_IN_CONTAINER\n"
        print("P8ED_ACTUAL_FILE_EDITOR_SIGKILL_MQTT_STILL_ALIVE_AUTH_RETAINED_PASS",flush=True)

        cmd(["podman","rm",editor],timeout=25)
        start_editor()
        inspect_live(editor,editor_data)
        assert cmd(["podman","exec",editor,"cat","/data/p8ed_real_editor_writer.txt"]).stdout.strip()=="P8ED_EDITOR_WRITTEN_IN_CONTAINER"
        assert retained_subscribe().stdout.strip()==payload
        print("P8ED_REAL_EDITOR_REPLACEMENT_REATTACH_DURABLE_DATA_WITH_MQTT_RUNNING_PASS",flush=True)
    finally:
        for name in (editor,mqtt):
            cmd(["podman","rm","--force",name],success=False,timeout=45)
    assert all(marker.read_bytes()==APP_MARKERS[slug][1] for marker,slug in zip(markers,slugs))
    assert editor_saved.read_text()=="P8ED_EDITOR_WRITTEN_IN_CONTAINER\n"
    assert all(sha256(src)==digest for src,digest in zip(archives,digests))
    remaining=cmd(["podman","ps","-a","--format","{{.Names}}"]).stdout.splitlines()
    assert not any(name.startswith("p8ed-real-") for name in remaining)
    print("P8ED_SOURCE_ARCHIVES_UNCHANGED_BOTH_NATIVE_APP_STATES_PRIVATE_CLEAN_TEARDOWN_PASS",flush=True)
    print("P8ED_LAB_TWO_REAL_HAOS_APPS_CONCURRENT_NOT_PRODUCTION_READY",flush=True)
