#!/usr/bin/env python3
"""P8dr: actual HAOS Mosquitto Supervisor backup -> native Rust SHA escrow
-> real upstream broker with retained MQTT data in one disposable rootless OCI.

This is a *compatibility feasibility* experiment, NOT restoration of HAOS
Supervisor configuration, MQTT user credentials, TLS or networking. Never
open host ports, never permit external network or access home services.
"""
import argparse, ast, hashlib, json, os, re, subprocess, tempfile, time
from pathlib import Path
from inspect_ha_backup import inspect_backup

p=argparse.ArgumentParser()
p.add_argument("--archive",required=True,type=Path)
p.add_argument("--manifest",required=True,type=Path)
p.add_argument("--native",required=True,type=Path)
a=p.parse_args()
assert os.environ.get("GITHUB_ACTIONS")=="true" and os.environ.get("RUNNER_TEMP")
assert os.geteuid()!=0,"real root forbidden"
assert subprocess.run(["podman","info","--format","{{.Host.Security.Rootless}}"],
     check=True,text=True,capture_output=True).stdout.strip()=="true"

archive=a.archive.resolve(strict=True)
native=a.native.resolve(strict=True)
manifest=json.loads(a.manifest.read_text())
assert archive.stat().st_size==manifest["size_bytes"]
with archive.open("rb") as handle:
    digest=hashlib.file_digest(handle,"sha256").hexdigest()
assert digest==manifest["sha256"]

source=Path(__file__).with_name("p8do_official_mosquitto_qemu_pytest.py")
marker=None
for node in ast.parse(source.read_text()).body:
    if isinstance(node,ast.FunctionDef) and node.name=="test_hars_official_mosquitto_app_only_partial_export":
        for child in node.body:
            if isinstance(child,ast.Assign) and any(
                isinstance(t,ast.Name) and t.id=="marker_value" for t in child.targets):
                marker=ast.literal_eval(child.value)
assert isinstance(marker,bytes) and marker.startswith(b"P8DO_REAL") and len(marker)<256
assert inspect_backup(archive,{"core_mosquitto"})["app_only_partial_archive_identified"]

IMAGE="docker.io/library/eclipse-mosquitto:2.0.22"
def cmd(argv,ok=True,timeout=95,env=None):
    x=subprocess.run(argv,check=False,text=True,capture_output=True,timeout=timeout,env=env)
    if ok:
        assert x.returncode==0, (argv[:4],x.returncode,x.stderr[-600:])
    return x

cmd(["podman","pull","--quiet",IMAGE],timeout=250)
image_id=cmd(["podman","image","inspect",IMAGE,"--format","{{.Id}}"]).stdout.strip()
assert re.fullmatch(r"(?:sha256:)?[0-9a-f]{64}", image_id), "invalid immutable OCI image ID"
assert cmd(["podman","image","inspect",IMAGE,"--format","{{.Architecture}}"]).stdout.strip()=="amd64"
print("P8DR_UPSTREAM_REAL_MQTT_BROKER_IMAGE_DIGEST_PINNED_IN_DISPOSABLE_CI_PASS",flush=True)

with tempfile.TemporaryDirectory(prefix="p8dr-mqtt-native-",dir=os.environ["RUNNER_TEMP"]) as t:
    root=Path(t)
    escrow=root/"escrow";escrow.mkdir(mode=0o700)
    target=root/"stage";target.mkdir(mode=0o700)
    config=root/"config";config.mkdir(mode=0o700)
    def native_call(*tokens):
        result=cmd([str(native),"--lab-readonly",str(escrow),*map(str,tokens)],timeout=125)
        value=json.loads(result.stdout)
        assert value["production_ready"] is False
        assert marker.decode(errors="ignore") not in result.stdout
        return value
    assert native_call("app-source-ingest",archive)["sha256"]==digest
    assert native_call("app-source-verify",digest)["archive_sha256"]==digest
    receipt=native_call("app-source-stage-inactive",digest,"core_mosquitto",target)
    assert receipt["files"]>=1 and not receipt["app_runtime_started"]
    staged=target/("app-core_mosquitto-"+digest)
    assert staged.is_dir()
    marker_path=staged/"data/rusty_home_p8do_nonsecret_mqtt_marker.txt"
    assert marker_path.is_file() and marker_path.read_bytes()==marker
    data=staged/"data"
    assert data.is_dir() and data.stat().st_mode&0o077==0
    assert len(list(target.iterdir()))==1
    print("P8DR_REAL_SUPERVISOR_MOSQUITTO_BACKUP_TO_NATIVE_RUST_VERIFIED_DATA_PASS",flush=True)

    # Not the HAOS add-on config. This is an EXPLICITLY NEW disposable
    # loopback-only config, to isolate feasibility of generic Mosquitto
    # binary running on staged storage. Never claim native config parity.
    configuration=config/"mosquitto.conf"
    configuration.write_text(
        "listener 1883 127.0.0.1\n"
        "allow_anonymous true\n"
        "persistence true\n"
        "persistence_location /mosquitto/data/\n"
        "persistence_file p8dr-mqtt-retained.db\n"
        "autosave_interval 1\n"
        "log_dest stdout\n"
    )
    configuration.chmod(0o600)
    name="p8dr-official-mosquitto-private-only"
    topic="p8dr/realhaos/native-retained"
    payload="P8DR_REAL_BROKER_RETAINED_QOS1_FICTIONAL_20261027"
    uid=os.getuid()
    gid=os.getgid()
    def launch():
        x=cmd(["podman","run","--detach","--replace","--name",name,
                "--userns=keep-id","--user",str(uid)+":"+str(gid),
                "--network","none","--read-only",
                "--cap-drop","ALL","--security-opt","no-new-privileges",
                "--pids-limit","80","--memory","256m",
                "--tmpfs","/tmp:rw,size=32m,mode=1777",
                "--mount",f"type=bind,source={config},destination=/mosquitto/config,readonly=true",
                "--mount",f"type=bind,source={data},destination=/mosquitto/data,rw=true",
                image_id],timeout=70)
        assert x.stdout.strip(),"official image did not launch"
        assert cmd(["podman","inspect",name,"--format","{{.HostConfig.NetworkMode}}"]).stdout.strip()=="none"
        assert cmd(["podman","inspect",name,"--format","{{.HostConfig.Privileged}}"]).stdout.strip()=="false"
        assert cmd(["podman","inspect",name,"--format","{{.HostConfig.ReadonlyRootfs}}"]).stdout.strip()=="true"
        assert cmd(["podman","port",name]).stdout.strip()==""
        m=json.loads(cmd(["podman","inspect",name,"--format","{{json .Mounts}}"]).stdout)
        assert any(x["Destination"]=="/mosquitto/data" and
                   x["Source"]==str(data) for x in m)
        return x.stdout.strip()
    def exec_mqtt(binary,*args,timeout=20,ok=True):
        return cmd(["podman","exec",name,binary,*args],ok=ok,timeout=timeout)
    def publish():
        x=None
        for attempt in range(35):
            x=exec_mqtt("mosquitto_pub","-h","127.0.0.1","-p","1883",
                        "-q","1","-r","-t",topic,"-m",payload,ok=False,timeout=6)
            if x.returncode==0:
                return
            time.sleep(.35)
        logs=cmd(["podman","logs",name],ok=False,timeout=10)
        raise AssertionError("Real MQTT listener never started: "+
                             (logs.stdout+logs.stderr)[-500:])
    def subscribe():
        reply=exec_mqtt("mosquitto_sub","-h","127.0.0.1","-p","1883",
                       "-C","1","-W","12","-t",topic,timeout=22)
        assert reply.stdout.strip()==payload, "not actual retained MQTT payload"
    try:
        first=launch()
        publish()
        subscribe()
        print("P8DR_REAL_MQTT_QOS1_PUBLISH_SUBSCRIBE_RETAINED_ON_RESTORED_NATIVE_DATA_PASS",flush=True)
        # Exit normally to force durable persistence.
        cmd(["podman","stop","--time","8",name],timeout=25)
        assert cmd(["podman","inspect",name,"--format","{{.State.Running}}"]).stdout.strip()=="false"
        db=data/"p8dr-mqtt-retained.db"
        assert db.is_file() and db.stat().st_size>0, "broker did not persist retained messages"
        assert marker_path.read_bytes()==marker
        cmd(["podman","rm",name],timeout=25)
        second=launch()
        assert second!=first,"Expected a genuinely NEW OCI instance"
        for attempt in range(35):
            test=exec_mqtt("mosquitto_sub","-h","127.0.0.1","-p","1883",
                    "-C","1","-W","1","-t",topic,ok=False,timeout=6)
            if test.returncode==0 and test.stdout.strip()==payload:
                break
            time.sleep(.35)
        else:
            raise AssertionError("Retained MQTT bytes lost after new official broker OCI instance")
        assert marker_path.read_bytes()==marker
        print("P8DR_REAL_MQTT_RETAINED_MESSAGE_DURABLE_AFTER_NEW_OCI_INSTANCE_PASS",flush=True)
    finally:
        cmd(["podman","rm","--force",name],ok=False,timeout=45)
    assert marker_path.read_bytes()==marker
    with archive.open("rb") as handle:
        assert hashlib.file_digest(handle,"sha256").hexdigest()==digest
    print("P8DR_ROOTLESS_NO_NETWORK_NO_HOST_PORT_ORIGINAL_HAOS_ARCHIVE_UNCHANGED_PASS",flush=True)
