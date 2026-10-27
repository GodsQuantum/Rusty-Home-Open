#!/usr/bin/env python3
"""P8dw: real mTLS on actual HAOS Mosquitto backup -> native Rust SHA escrow and LOCAL AUTH
-> real upstream broker with retained MQTT data in one disposable rootless OCI.

This is a *compatibility feasibility* experiment, NOT restoration of HAOS
Supervisor configuration, MQTT user credentials, TLS or networking. Never
open host ports, never permit external network or access home services.
"""
import argparse, ast, hashlib, json, os, re, stat, subprocess, tempfile, time
from haos_mosquitto_local_auth_lab import generate, check_options, MigrationRefused
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

    # DOCUMENTED HAOS Mosquitto 7.1.1 options schema; only SYNTHETIC LOCAL
    # user here, NOT real HAOS identity secrets or Core auth delegation.
    # The genuine selected-App-only archive remains the source for /data.
    user="p8dt_ci_local"
    password="P8DT_TEST_ONLY_SYNTHETIC_SECRET_WITH_32_CHARS"
    options={
        "logins":[{"username":user,"password":password}],
        "log_dest":["stdout"],"log_type":["error","warning","notice"],
        "require_certificate":False,
        "certfile":"fullchain.pem","keyfile":"privkey.pem",
        "customize":{"active":False,"folder":"mosquitto"},
    }
    forbidden=[
      dict(options,logins=[]),
      dict(options,require_certificate=True),
      dict(options,cafile="ca.pem"),
      dict(options,customize={"active":True,"folder":"mosquitto"}),
      dict(options,logins=[{"username":"homeassistant","password":password}]),
      dict(options,logins=[{"username":user,"password":"pbkdf2fake", "password_pre_hashed":True}]),
      dict(options,logins=[{"username":user,"password":"weak"}]),
      dict(options,anonymous=True),
      dict(options,debug=True),
      dict(options,certfile="../../leak"),
    ]
    for bad in forbidden:
        try:
            check_options(bad)
        except MigrationRefused:
            assert not list(config.iterdir())
        else:
            raise AssertionError("Official HAOS Mosquitto unsafe option unexpectedly admitted")
    print("P8DT_SUPERVISOR_UNKNOWN_UNMAPPED_TLS_DELEGATED_AUTH_OPTIONS_REFUSED_PASS",flush=True)
    report=generate(options,config,image_id)
    assert report["local_logins"]==1 and report["anonymous_allowed"] is False
    assert report["auth_delegate_migrated"] is False
    assert report["tls_migrated"] is False and report["production_ready"] is False
    assert (config/"mosquitto.conf").stat().st_mode & 0o077==0
    assert (config/"passwords").stat().st_mode & 0o077==0
    assert password not in (config/"mosquitto.conf").read_text()
    assert password not in (config/"passwords").read_text()
    assert "allow_anonymous false" in (config/"mosquitto.conf").read_text()
    print("P8DT_OFFICIAL_SUPERVISOR_LOCAL_OPTION_FORMAT_TO_PRIVATE_UPSTREAM_PASSWORD_HASH_PASS",flush=True)
    # New P8dw: ephemeral CA, server cert and client cert (NEVER household
    # keys, no reuse). Pure offline OpenSSL inside disposable GitHub runner.
    certs=root/"certificates"
    certs.mkdir(mode=0o700)
    def ca(name):
        cmd(["openssl","req","-x509","-newkey","rsa:2048","-sha256",
             "-nodes","-days","1","-keyout",str(certs/(name+".key")),
             "-out",str(certs/(name+".pem")),
             "-subj","/CN=rustyhome-ci-"+name,
             "-addext","basicConstraints=critical,CA:TRUE",
             "-addext","keyUsage=critical,keyCertSign,cRLSign"],timeout=80)
    def leaf(name,issuer,eku):
        cert=certs/(name+".pem")
        csr=certs/(name+".csr")
        cmd(["openssl","req","-new","-newkey","rsa:2048","-sha256","-nodes",
             "-keyout",str(certs/(name+".key")),"-out",str(csr),
             "-subj","/CN="+("localhost" if name=="server" else name),
             "-addext","extendedKeyUsage="+eku,
             "-addext","subjectAltName=IP:127.0.0.1" if name=="server" else
                       "subjectAltName=DNS:"+name+".ci.invalid"],timeout=80)
        cmd(["openssl","x509","-req","-in",str(csr),
             "-CA",str(certs/(issuer+".pem")),"-CAkey",str(certs/(issuer+".key")),
             "-CAcreateserial","-out",str(cert),"-days","1","-sha256",
             "-copy_extensions","copy"],timeout=60)
        cmd(["openssl","verify","-CAfile",str(certs/(issuer+".pem")),
             "-purpose","sslserver" if eku=="serverAuth" else "sslclient",
             str(cert)],timeout=25)
        csr.unlink()
    ca("test_ca");ca("untrusted_ca")
    leaf("server","test_ca","serverAuth")
    leaf("client","test_ca","clientAuth")
    leaf("rogue_client","untrusted_ca","clientAuth")
    for item in certs.iterdir():
        assert item.is_file() and not item.is_symlink()
        item.chmod(0o600)
    assert all((f.stat().st_mode&0o077)==0 for f in certs.iterdir())
    base=(config/"mosquitto.conf").read_text()
    assert base.count("listener 1883 127.0.0.1")==1
    assert "allow_anonymous false" in base and "password_file /mosquitto/config/passwords" in base
    # Replace only loopback plaintext listener; TLS is REQUIRED on port 8883.
    mtls=base.replace("listener 1883 127.0.0.1","listener 8883 127.0.0.1")
    mtls+=(
        "cafile /mosquitto/certs/test_ca.pem\n"
        "certfile /mosquitto/certs/server.pem\n"
        "keyfile /mosquitto/certs/server.key\n"
        "require_certificate true\n"
        "use_identity_as_username false\n"
        "tls_version tlsv1.2\n"
    )
    assert "listener 1883" not in mtls
    assert "require_certificate true" in mtls and "allow_anonymous false" in mtls
    (config/"mosquitto.conf").write_text(mtls)
    print("P8DW_EPHEMERAL_CA_SERVER_CLIENT_CERT_CHAIN_OPENSSL_VERIFIED_PASS",flush=True)
    name="p8dw-official-mosquitto-mtls"
    topic="p8dw/mtls-authenticated/retained"
    payload="P8DW_REAL_MTLS_MQTT_RETAINED_FICTIONAL_20261027"
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
                "--mount",f"type=bind,source={certs},destination=/mosquitto/certs,readonly=true",
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
    def exec_mqtt(binary,*args,timeout=20,ok=True,tls_mode="authorized"):
        # Genuine Mosquitto clients verify real CA trust and client cert.
        # Never use --insecure or bypass hostname verification.
        certs_args=["--cafile","/mosquitto/certs/test_ca.pem",
                    "--cert","/mosquitto/certs/client.pem",
                    "--key","/mosquitto/certs/client.key"]
        if tls_mode=="no_client_cert":
            certs_args=["--cafile","/mosquitto/certs/test_ca.pem"]
        elif tls_mode=="untrusted_ca":
            certs_args=["--cafile","/mosquitto/certs/untrusted_ca.pem",
                        "--cert","/mosquitto/certs/client.pem",
                        "--key","/mosquitto/certs/client.key"]
        elif tls_mode=="untrusted_client":
            certs_args=["--cafile","/mosquitto/certs/test_ca.pem",
                        "--cert","/mosquitto/certs/rogue_client.pem",
                        "--key","/mosquitto/certs/rogue_client.key"]
        else:
            assert tls_mode=="authorized"
        return cmd(["podman","exec",name,binary,*args,*certs_args],ok=ok,timeout=timeout)
    def publish():
        x=None
        for attempt in range(35):
            x=exec_mqtt("mosquitto_pub","-h","127.0.0.1","-p","8883",
                        "-u",user,"-P",password,"-q","1","-r","-t",topic,"-m",payload,ok=False,timeout=6)
            if x.returncode==0:
                return
            time.sleep(.35)
        logs=cmd(["podman","logs",name],ok=False,timeout=10)
        raise AssertionError("Real MQTT listener never started: "+
                             (logs.stdout+logs.stderr)[-500:])
    def subscribe():
        reply=exec_mqtt("mosquitto_sub","-h","127.0.0.1","-p","8883",
                       "-u",user,"-P",password,"-C","1","-W","12","-t",topic,timeout=22)
        assert reply.stdout.strip()==payload, "not actual retained MQTT payload"
    try:
        first=launch()
        publish()
        subscribe()
        # Actual mosquitto protocol calls with NO credentials and with
        # deliberately WRONG password must be denied. Never only grep config.
        unauth=exec_mqtt("mosquitto_pub","-h","127.0.0.1","-p","8883",
                    "-t",topic,"-m","no-auth-should-not-pass",ok=False,timeout=12)
        assert unauth.returncode!=0
        wrong=exec_mqtt("mosquitto_pub","-h","127.0.0.1","-p","8883",
                    "-u",user,"-P","P8DT_THIS_IS_WRONG_PASSWORD",
                    "-t",topic,"-m","bad-auth-should-not-pass",ok=False,timeout=12)
        assert wrong.returncode!=0
        subscribe()
        # Real TLS failures, not config greps or only a stopped container.
        # For each test we keep a correct MQTT password to isolate TLS.
        no_client=exec_mqtt("mosquitto_pub","-h","127.0.0.1","-p","8883",
                    "-u",user,"-P",password,"-t",topic,"-m","missing-client-cert",
                    tls_mode="no_client_cert",ok=False,timeout=12)
        assert no_client.returncode!=0
        rogue_client=exec_mqtt("mosquitto_pub","-h","127.0.0.1","-p","8883",
                    "-u",user,"-P",password,"-t",topic,"-m","untrusted-client",
                    tls_mode="untrusted_client",ok=False,timeout=12)
        assert rogue_client.returncode!=0
        wrong_server_ca=exec_mqtt("mosquitto_pub","-h","127.0.0.1","-p","8883",
                    "-u",user,"-P",password,"-t",topic,"-m","untrusted-server",
                    tls_mode="untrusted_ca",ok=False,timeout=12)
        assert wrong_server_ca.returncode!=0
        subscribe()
        print("P8DW_REAL_MQTT_MTLS_ABSENT_ROGUE_CLIENT_AND_UNTRUSTED_SERVER_CA_DENIED_PASS",flush=True)
        print("P8DT_REAL_NATIVE_MQTT_GOOD_LOGIN_ACCEPTED_ANONYMOUS_AND_BAD_DENIED_PASS",flush=True)
        print("P8DT_REAL_MQTT_QOS1_PUBLISH_SUBSCRIBE_RETAINED_ON_RESTORED_NATIVE_DATA_PASS",flush=True)
        # Exit normally to force durable persistence.
        cmd(["podman","stop","--time","8",name],timeout=25)
        assert cmd(["podman","inspect",name,"--format","{{.State.Running}}"]).stdout.strip()=="false"
        db=data/"p8dt-auth-retained.db"
        assert db.is_file() and db.stat().st_size>0, "broker did not persist retained messages"
        assert marker_path.read_bytes()==marker
        cmd(["podman","rm",name],timeout=25)
        second=launch()
        assert second!=first,"Expected a genuinely NEW OCI instance"
        for attempt in range(35):
            test=exec_mqtt("mosquitto_sub","-h","127.0.0.1","-p","8883",
                    "-u",user,"-P",password,"-C","1","-W","1","-t",topic,ok=False,timeout=6)
            if test.returncode==0 and test.stdout.strip()==payload:
                break
            time.sleep(.35)
        else:
            raise AssertionError("Retained MQTT bytes lost after new official broker OCI instance")
        assert marker_path.read_bytes()==marker
        print("P8DW_REAL_MTLS_MQTT_RETAINED_AFTER_NEW_OCI_PASS",flush=True)
    finally:
        cmd(["podman","rm","--force",name],ok=False,timeout=45)
    assert marker_path.read_bytes()==marker
    with archive.open("rb") as handle:
        assert hashlib.file_digest(handle,"sha256").hexdigest()==digest
    print("P8DW_REAL_HAOS_SOURCE_SHA_NO_HOST_PORT_MTLS_TEARDOWN_PASS",flush=True)
