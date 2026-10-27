#!/usr/bin/env python3
"""P8dw guard for genuine TLS client/server auth on real HAOS native files."""
from pathlib import Path
r=Path(__file__).resolve().parents[1]
w=(r/'.github/workflows/p8dw-mqtt-mtls-official-haos.yml').read_text()
s=(r/'scripts/test_p8dw_genuine_mqtt_mtls.py').read_text()
for text in (
  "haos_ova-18.3.qcow2.xz",
  "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
  "-snapshot -accel kvm",
  "test_p8do_real_mosquitto_native_import.py",
  "test_p8dt_mosquitto_supervisor_local_auth.py",
  "test_p8dw_genuine_mqtt_mtls.py",
  "P8DW_REAL_MQTT_MTLS_VALID_USER_CERT_PERSISTENCE_AND_DENIAL_PASS"):
  assert text in w,text
for text in (
  "openssl","rsa:2048","sslserver","sslclient",
  '"extendedKeyUsage="+eku', '"subjectAltName=IP:127.0.0.1"',
  "require_certificate true", "use_identity_as_username false",
  "allow_anonymous false","p8dw/mtls-authenticated/retained",
  '"--network","none"', '"--read-only"', '"--cap-drop","ALL"',
  '"--security-opt","no-new-privileges"',
  'destination=/mosquitto/certs,readonly=true',
  'mosquitto_pub','mosquitto_sub',
  'tls_mode="no_client_cert"', 'tls_mode="untrusted_ca"',
  'tls_mode="untrusted_client"',
  "P8DW_REAL_MQTT_MTLS_ABSENT_ROGUE_CLIENT_AND_UNTRUSTED_SERVER_CA_DENIED_PASS",
  "P8DW_REAL_MTLS_MQTT_RETAINED_AFTER_NEW_OCI_PASS",
  "P8DW_REAL_HAOS_SOURCE_SHA_NO_HOST_PORT_MTLS_TEARDOWN_PASS"):
  assert text in s,text
for banned in ("--network=host","--privileged","sudo podman",
 "pct exec","qm stop 100","192.168.","pull_request_target:","workflow_dispatch:"):
  assert banned not in w+s,banned
assert "GITHUB_ACTIONS" in s
assert "mosquitto_pub" in s and "mosquitto_sub" in s
assert "config/\"mosquitto.conf\"" in s
print("P8DW_REAL_TLS_CA_AND_MUTUAL_AUTH_FAIL_CLOSED_NO_HOST_EXPOSURE_SCOPE_PASS")
