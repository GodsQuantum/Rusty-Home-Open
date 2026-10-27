#!/usr/bin/env bash
# Eighth genuine distinct stateful OCI App: official Qdrant vector points.
# Disposable rootless GitHub Actions runner only, no home network/VM100.
set -euo pipefail
[[ "$(printenv GITHUB_ACTIONS || true)" == true && -n "$(printenv RUNNER_TEMP || true)" && "$(id -u)" != 0 ]] || exit 2
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export XDG_RUNTIME_DIR="/run/user/$(id -u)"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
test -S "$XDG_RUNTIME_DIR/bus"
BIN="$ROOT/target/debug/oci_app_manager_lab"
test -x "$BIN"
QD="$HOME/.config/containers/systemd"
source_slug=vector-source
restore_slug=vector-restore
source_vol=rh-vector-source-data
restore_vol=rh-vector-restore-data
test ! -e "$QD/rh-$source_slug.container"
test ! -e "$QD/rh-$restore_slug.container"
! podman volume exists "$source_vol"
! podman volume exists "$restore_vol"
WORK="$(mktemp -d "$(printenv RUNNER_TEMP)/p8cf-qdrant-XXXXXX")"
chmod 0700 "$WORK"
cleanup() {
 HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$restore_slug" >/dev/null 2>&1 || true
 HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$source_slug" >/dev/null 2>&1 || true
 podman volume rm "$restore_vol" "$source_vol" >/dev/null 2>&1 || true
 rm -rf "$WORK"
}
trap cleanup EXIT

# Official verified upstream v1.20.0 UNPRIVILEGED image; the mutable tag
# is used ONLY for pulling, never in a stored Quadlet/OCI installation.
podman pull --quiet ghcr.io/qdrant/qdrant/qdrant:v1.20.0-unprivileged
IMAGE="$(podman image inspect ghcr.io/qdrant/qdrant/qdrant:v1.20.0-unprivileged --format '{{index .RepoDigests 0}}')"
[[ "$IMAGE" == ghcr.io/qdrant/qdrant/qdrant@sha256:* ]]
podman image exists "$IMAGE"
podman pull --quiet docker.io/library/python:3.12-alpine
PY_IMAGE="$(podman image inspect docker.io/library/python:3.12-alpine --format '{{index .RepoDigests 0}}')"
[[ "$PY_IMAGE" == docker.io/library/python@sha256:* ]]

# Upstream Qdrant binary and config untouched. Isolated offline UID 1000,
# native persistent storage and immutable image; no host port publication.
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-path-lab  "$source_slug" "$IMAGE" 1000 /qdrant/storage
test "$(systemctl --user is-active rh-$source_slug.service)" = inactive
test "$(stat -c %a "$QD/rh-$source_slug.container")" = 600
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$source_slug"
assert_offline() {
 test "$(podman inspect "$1" --format '{{.HostConfig.NetworkMode}}')" = none
 test "$(podman inspect "$1" --format '{{.HostConfig.ReadonlyRootfs}}')" = true
 test "$(podman inspect "$1" --format '{{.Config.User}}')" = 1000
 test "$(podman inspect "$1" --format '{{range .Mounts}}{{if eq .Destination "/qdrant/storage"}}{{.Name}}{{end}}{{end}}')" = "$2"
}
probe() {
 local container="$1" action="$2"
 # The read-only official Python utility shares only the ALREADY Network=none
 # namespace; it does not open internet/public ports or host-level networking.
 podman run --rm --network "container:$container" --user 65534   --read-only --cap-drop ALL --security-opt no-new-privileges   --pids-limit 32 --memory 96m "$PY_IMAGE" python -B -c '
import json,sys,time,urllib.request,urllib.error
action=sys.argv[1]
base="http://127.0.0.1:6333"
collection="p8cf_offline_vector_fixture"
one={"id":1,"vector":[0.2,0.4,0.8],"payload":{"checkpoint":"before","value":42}}
two={"id":2,"vector":[0.6,0.3,0.1],"payload":{"checkpoint":"after","value":99}}
def request(method,path,body=None):
 data=None if body is None else json.dumps(body,separators=(",",":")).encode()
 req=urllib.request.Request(base+path,method=method,data=data,headers={"Content-Type":"application/json"})
 with urllib.request.urlopen(req,timeout=12) as r:
  if r.status not in (200,201,202):raise RuntimeError("bad status "+str(r.status))
  return json.load(r)
def check(expected):
 response=request("POST","/collections/"+collection+"/points",
                  {"ids":[1,2],"with_payload":True,"with_vector":True})
 assert response["status"]=="ok",response
 points=response["result"]
 assert {item["id"] for item in points}==set(expected),points
 for item in points:
  obj=one if item["id"]==1 else two
  assert item["payload"]==obj["payload"],item
  assert len(item["vector"])==3,item
  assert all(abs(a-b)<1e-5 for a,b in zip(item["vector"],obj["vector"])),item
if action=="init":
 response=request("PUT","/collections/"+collection,
                  {"vectors":{"size":3,"distance":"Dot"}})
 assert response["status"]=="ok" and response["result"] is True,response
if action in ("init","after"):
 item=one if action=="init" else two
 response=request("PUT","/collections/"+collection+"/points?wait=true",
                  {"points":[item]})
 assert response["status"]=="ok" and response["result"]["status"]=="completed",response
check([1,2] if action in ("after","both") else [1])
print("P8CF_GENUINE_QDRANT_REST_VECTOR_"+action.upper()+"_PASS")
' "$action"
}
ready() {
 for _ in $(seq 1 90); do
  if podman run --rm --network "container:$1" --user 65534 --read-only    --cap-drop ALL --security-opt no-new-privileges --pids-limit 32    --memory 96m "$PY_IMAGE" python -B -c '
import urllib.request,sys
try:
 with urllib.request.urlopen("http://127.0.0.1:6333/healthz",timeout=2) as r:
  sys.exit(0 if r.status==200 else 1)
except Exception:sys.exit(1)' >/dev/null 2>&1; then return 0; fi
  sleep .25
 done
 podman logs "$1" 2>&1 | tail -40 >&2 || true
 return 1
}
ready rh-vector-source
assert_offline rh-vector-source "$source_vol"
probe rh-vector-source init
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --stop-lab "$source_slug"
! podman ps --format '{{.Names}}' | grep -Fx rh-vector-source
umask 077
ARCHIVE="$WORK/qdrant-private.tar"
podman volume export --output "$ARCHIVE" "$source_vol"
chmod 0600 "$ARCHIVE"
test -s "$ARCHIVE"
test "$(stat -c %a "$ARCHIVE")" = 600
tar -tf "$ARCHIVE" > "$WORK/volume-manifest.txt"
grep -Eq '(^|/)collections/' "$WORK/volume-manifest.txt"
SHA="$(sha256sum "$ARCHIVE" | cut -d' ' -f1)"
[[ "$SHA" =~ ^[0-9a-f]{64}$ ]]
echo "$SHA  $ARCHIVE" | sha256sum -c
cp "$ARCHIVE" "$WORK/bad.tar"
printf CORRUPTED >> "$WORK/bad.tar"
if echo "$SHA  $WORK/bad.tar" | sha256sum -c >/dev/null 2>&1; then exit 1; fi
echo P8CF_QDRANT_STOPPED_REAL_VECTOR_WAL_VOLUME_SHA_TAMPER_REFUSED_PASS

HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$source_slug"
ready rh-vector-source
probe rh-vector-source first
probe rh-vector-source after
echo P8CF_QDRANT_SOURCE_COMMITTED_SECOND_VECTOR_POST_BACKUP_PASS

! podman volume exists "$restore_vol"
podman volume create "$restore_vol" >/dev/null
echo "$SHA  $ARCHIVE" | sha256sum -c
podman volume import "$restore_vol" "$ARCHIVE"
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-path-lab  "$restore_slug" "$IMAGE" 1000 /qdrant/storage
test "$(systemctl --user is-active rh-$restore_slug.service)" = inactive
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$restore_slug"
ready rh-vector-restore
assert_offline rh-vector-restore "$restore_vol"
probe rh-vector-restore first
probe rh-vector-source both
echo P8CF_EIGHTH_REAL_VECTOR_DATABASE_INDEPENDENT_RESTORE_POINT_IN_TIME_PASS

HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$restore_slug"
podman volume exists "$restore_vol"
probe rh-vector-source both
echo P8CF_RESTORED_VECTOR_APP_REMOVAL_PRESERVES_SOURCE_AND_CHECKPOINT_PASS
