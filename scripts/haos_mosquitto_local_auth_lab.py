#!/usr/bin/env python3
"""Fail-closed LAB adapter: documented Supervisor Mosquitto LOCAL logins only.

Only translates a narrowly defined HAOS options JSON subset to an OFFLINE
temporary native Mosquitto config. Absolutely DOES NOT convert HA users,
Supervisor auth API delegation, pre-hashed HA auth-plugin passwords,
TLS certificate material, custom ACLs/overrides or production secrets.

Local password hashing delegates to the genuine upstream OCI
mosquitto_passwd INTERACTIVE PTY so plaintext is NEVER in process argv,
shell history, build logs or Mosquitto config. Not a production daemon.
"""
import json
import os
import re
import stat
from pathlib import Path
import pexpect

ALLOWED = {"logins", "log_dest", "log_type", "require_certificate",
           "certfile", "keyfile", "cafile", "customize", "debug"}
NAME=re.compile(r"[A-Za-z0-9_.-]{1,64}",flags=re.ASCII)
RESERVED={"homeassistant","addons"}
LOG_DEST={"stdout"}
LOG_TYPE={"error","warning","notice","information"}
IMAGE="docker.io/library/eclipse-mosquitto:2.0.22"

class MigrationRefused(ValueError):
    pass

def require_private(path):
    info=path.lstat()
    if not path.is_dir() or path.is_symlink() or info.st_uid!=os.geteuid() or info.st_mode & 0o077:
        raise MigrationRefused("owned private directory required")

def check_options(raw):
    if not isinstance(raw,dict) or set(raw)-ALLOWED:
        raise MigrationRefused("unknown or invalid Supervisor Mosquitto option")
    if raw.get("require_certificate") is not False:
        raise MigrationRefused("TLS/mTLS Supervisor files not yet mapped or verified")
    if "cafile" in raw and raw["cafile"]:
        raise MigrationRefused("CA/client certificate migration not certified")
    # Certificate filenames may be installed as defaults in Supervisor even
    # when the corresponding SSL files are absent; do not claim TLS migrated.
    if raw.get("certfile") not in (None,"fullchain.pem") or raw.get("keyfile") not in (None,"privkey.pem"):
        raise MigrationRefused("custom TLS certificates not imported")
    customization=raw.get("customize",{})
    if not isinstance(customization,dict) or customization.get("active") is not False or customization.get("folder") not in (None,"mosquitto") or set(customization)-{"active","folder"}:
        raise MigrationRefused("custom Supervisor Mosquitto ACL or includes unsupported")
    if raw.get("debug") not in (None,False):
        raise MigrationRefused("unsafe debug logging in native broker")
    log_dest=raw.get("log_dest",[])
    if log_dest not in ([],["stdout"]):
        raise MigrationRefused("nonstdout Supervisor logging not migrated")
    log_type=raw.get("log_type",[])
    if not isinstance(log_type,list) or set(log_type)-LOG_TYPE:
        raise MigrationRefused("unsupported Supervisor log type")
    logins=raw.get("logins")
    if not isinstance(logins,list) or not 1<=len(logins)<=12:
        # Empty logins would enable auth via official Supervisor HTTP
        # backend, WHICH IS NOT MIGRATED. DO NOT allow anonymous fallback.
        raise MigrationRefused("no local logins: HA auth delegation unsupported")
    normalized=[]
    seen=set()
    for x in logins:
        if not isinstance(x,dict) or set(x)-{"username","password","password_pre_hashed"}:
            raise MigrationRefused("unknown Supervisor local login option")
        username=x.get("username")
        password=x.get("password")
        if not isinstance(username,str) or NAME.fullmatch(username) is None or username.casefold() in RESERVED or username.casefold() in seen:
            raise MigrationRefused("invalid, reserved or duplicate MQTT login")
        if x.get("password_pre_hashed",False) is not False:
            # HA auth-plugin PBKDF2 is not Mosquitto's password_file format.
            raise MigrationRefused("hashed HAOS auth-plugin password not validated for Mosquitto")
        if not isinstance(password,str) or not 16<=len(password)<=256 or any(ord(c)<32 for c in password):
            raise MigrationRefused("unsafe or undersized MQTT local secret")
        seen.add(username.casefold())
        normalized.append((username,password))
    return normalized

def generate(options, destination, image_id):
    require_private(destination)
    if list(destination.iterdir()):
        raise MigrationRefused("refuse existing native broker configuration")
    if not isinstance(image_id,str) or not re.fullmatch(r"(?:sha256:)?[0-9a-f]{64}",image_id):
        raise MigrationRefused("immutable upstream image ID required")
    logins=check_options(options) # fail before creating any files
    password_file=destination/"passwords"
    # Requires upstream mosquitto_passwd, never passes passwords via argv.
    for i,(name,password) in enumerate(logins):
        args=["run","--rm","-it","--network","none",
            "--userns=keep-id","--user",str(os.geteuid())+":"+str(os.getegid()),
            "--security-opt","no-new-privileges","--cap-drop","ALL",
            "--mount",f"type=bind,source={destination},destination=/mosquitto/config,rw=true",
            "--entrypoint","mosquitto_passwd",image_id]
        if i==0:
            args+=["-c"]
        args+=["/mosquitto/config/passwords",name]
        p=pexpect.spawn("podman",args,encoding="utf-8",timeout=35,echo=False)
        try:
            p.expect([r"(?i)Password:",r"(?i)Enter password:"],timeout=40)
            p.sendline(password)
            p.expect([r"(?i)Reenter password:",r"(?i)Confirm password:"],timeout=40)
            p.sendline(password)
            p.expect(pexpect.EOF,timeout=55)
            p.close()
            if p.exitstatus!=0:
                raise MigrationRefused("upstream Mosquitto password hashing failed")
        finally:
            if p.isalive():
                p.terminate(force=True)
    st=password_file.lstat()
    if not stat.S_ISREG(st.st_mode) or st.st_nlink!=1 or st.st_uid!=os.geteuid():
        raise MigrationRefused("bad generated password file inode")
    password_file.chmod(0o600)
    contents=password_file.read_text()
    assert all(len(line.split(":",1))==2 and line.split(":",1)[1].startswith(("$7$","$6$","$argon2id$")) for line in contents.splitlines())
    assert len(contents.splitlines())==len(logins)
    assert all(secret not in contents for _,secret in logins)
    config=destination/"mosquitto.conf"
    lines=[
      "listener 1883 127.0.0.1",
      "protocol mqtt",
      "allow_anonymous false",
      "password_file /mosquitto/config/passwords",
      "persistence true",
      "persistence_location /mosquitto/data/",
      "persistence_file p8dt-auth-retained.db",
      "autosave_interval 1",
      "log_dest stdout"
    ]
    config.write_text("\n".join(lines)+"\n")
    config.chmod(0o600)
    assert "allow_anonymous false" in config.read_text()
    assert all(secret not in config.read_text() for _,secret in logins)
    return {"local_logins":len(logins),"auth_delegate_migrated":False,
            "tls_migrated":False,"anonymous_allowed":False,"production_ready":False}
