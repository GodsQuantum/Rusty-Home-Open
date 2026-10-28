#!/usr/bin/env python3
"""P8ei actual official Mosquitto MQTT plugin lab; no host port or production.

Pinned real Home Assistant 7.1.1 app image and included go-auth.so, two local
Rust HTTP fixtures, genuine MQTT CONNECT protocol over network-less Docker.
Never read household credentials, VM100, Cloud9 production networks or tokens.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import uuid

OFFICIAL = ("homeassistant/amd64-addon-mosquitto@sha256:"
    "a52dd07cb946a37837864b681705834bd06fb15068a14155537504cf54271e5e")
PLUGIN = "/usr/share/mosquitto/go-auth.so"
CONF = """\
user root
log_dest stdout
log_type error
log_type warning
log_type notice
persistence false
auth_plugin /usr/share/mosquitto/go-auth.so
auth_plugin_deny_special_chars false
auth_opt_backends http
auth_opt_cache false
auth_opt_http_host 127.0.0.1
auth_opt_http_port 18083
auth_opt_http_getuser_uri /authentication
auth_opt_http_superuser_uri /superuser
auth_opt_http_aclcheck_uri /acl
auth_opt_http_response_mode status
auth_opt_http_params_mode json
auth_opt_http_timeout 1
auth_opt_http_method POST
auth_opt_log_level error
listener 18883 127.0.0.1
protocol mqtt
allow_anonymous false
"""

def run(args, timeout=40, check=True):
    p=subprocess.run(args,text=True,capture_output=True,timeout=timeout)
    if check and p.returncode:
        raise RuntimeError(f"failed ({p.returncode}): {' '.join(args[:5])}\n{p.stderr[-1100:]}")
    return p

def wait_probe(marker,args,attempts=25):
    last=None
    for _ in range(attempts):
        last=run(args,timeout=12,check=False)
        if last.returncode==0:
            print(marker,flush=True)
            return
        time.sleep(.65)
    raise RuntimeError(f"{marker} failed: {last.stderr[-850:]}")

def main(binary):
    binary=binary.resolve(strict=True)
    assert binary.is_file() and os.access(binary,os.X_OK)
    assert sys.platform=="linux"
    with tempfile.TemporaryDirectory(prefix="p8ei-official-mqtt-") as dirname:
        lab=Path(dirname)
        lab.chmod(0o700)
        executable=lab/"p8ei_actual_mqtt_lab"
        executable.write_bytes(binary.read_bytes())
        executable.chmod(0o755)
        (lab/"mosquitto.conf").write_text(CONF)
        (lab/"mosquitto.conf").chmod(0o644)
        control=lab/"control"
        control.write_text("permit\n")
        control.chmod(0o644)
        name="p8ei-real-goauth-"+uuid.uuid4().hex[:12]
        created=False
        try:
            run(["docker","pull",OFFICIAL],timeout=160)
            run(["docker","run","--rm","-d","--name",name,
                 "--network","none","--read-only","--cap-drop","ALL",
                 "--security-opt","no-new-privileges","--user","1000:1000",
                 "--tmpfs","/tmp:rw,nosuid,nodev,mode=1777",
                 "--mount",f"type=bind,src={lab},dst=/lab,readonly",
                 "--entrypoint","/bin/sh",OFFICIAL,"-c","sleep 360"],timeout=35)
            created=True
            info=json.loads(run(["docker","inspect",name]).stdout)[0]
            host=info["HostConfig"]
            assert host["NetworkMode"]=="none"
            assert host["ReadonlyRootfs"] is True
            assert "ALL" in host["CapDrop"]
            assert host["Privileged"] is False
            assert host["PortBindings"] in (None, {})
            print("P8EI_PINNED_OFFICIAL_IMAGE_NETWORK_NONE_READONLY_NO_HOST_PORT_PASS",flush=True)
            run(["docker","exec",name,"/bin/sh","-c",
                 f"test -f {PLUGIN} && test -r {PLUGIN} && command -v mosquitto"])
            print("P8EI_GENUINE_OFFICIAL_MOSQUITTO_GO_AUTH_SO_FOUND_PASS",flush=True)
            run(["docker","exec","-d",name,"/bin/sh","-c",
                 "/lab/p8ei_actual_mqtt_lab server /lab/control > /tmp/bridge.log 2>&1"])
            wait_probe("P8EI_REAL_TWO_HTTP_LOOPBACK_BOUNDARIES_READY_PASS",
                       ["docker","exec",name,"/lab/p8ei_actual_mqtt_lab","ready"])
            run(["docker","exec","-d",name,"/bin/sh","-c",
                 "mosquitto -c /lab/mosquitto.conf -v > /tmp/broker.log 2>&1"])
            wait_probe("P8EI_REAL_OFFICIAL_MQTT_PLUGIN_VALID_LOGIN_CONNACK_PASS",
                       ["docker","exec",name,"/lab/p8ei_actual_mqtt_lab","probe","valid"])
            for mode in ("wrong","anonymous","reserved"):
                r=run(["docker","exec",name,"/lab/p8ei_actual_mqtt_lab","probe",mode])
                assert "CONNACK_PASS" in r.stdout
                print(f"P8EI_REAL_MQTT_{mode.upper()}_LOGIN_DENIED_PASS",flush=True)
            control.write_text("deny\n")
            run(["docker","exec",name,"/lab/p8ei_actual_mqtt_lab","probe-denied-valid"])
            print("P8EI_REAL_MQTT_SAME_USERNAME_PASSWORD_REVOKED_DENIED_PASS",flush=True)
            control.write_text("outage\n")
            run(["docker","exec",name,"/lab/p8ei_actual_mqtt_lab","probe-denied-valid"])
            print("P8EI_REAL_MQTT_SUPERVISOR_HTTP_503_DENIED_NO_FALLBACK_PASS",flush=True)
            control.write_text("permit\n")
            run(["docker","exec",name,"/lab/p8ei_actual_mqtt_lab","probe","valid"])
            print("P8EI_REAL_MQTT_AUTHENTICATION_BACKEND_RECOVERY_PASS",flush=True)
            print("P8EI_OFFICIAL_GO_AUTH_ACTUAL_MQTT_MOCK_SUPERVISOR_NOT_PRODUCTION_READY",flush=True)
        except Exception:
            if created:
                detail=run(["docker","exec",name,"/bin/sh","-c",
                            "tail -25 /tmp/broker.log 2>/dev/null || true"],check=False)
                print("P8EI_BROKER_FIXTURE_LOG:",detail.stdout[-1800:],file=sys.stderr)
            raise
        finally:
            if created:
                run(["docker","rm","-f",name],timeout=30,check=False)
                assert run(["docker","inspect",name],check=False).returncode!=0
                print("P8EI_CONTAINER_FULL_CLEANUP_NO_HOST_PORTS_PASS",flush=True)

if __name__=="__main__":
    assert len(sys.argv)==2
    main(Path(sys.argv[1]))
