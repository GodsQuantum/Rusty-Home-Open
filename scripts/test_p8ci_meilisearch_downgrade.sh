#!/usr/bin/env bash
# P8ci: genuine Meilisearch downgrade refusal after a valid snapshot.
# Rootless GitHub runner only. No home/host ports, radios, production or secrets.
set -euo pipefail
[[ "$(printenv GITHUB_ACTIONS || true)" == true && -n "$(printenv RUNNER_TEMP || true)" && "$(id -u)" != 0 ]] || exit 2
[[ "$RUNNER_TEMP" == /home/runner/work/_temp* ]] || exit 2
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export XDG_RUNTIME_DIR="/run/user/$(id -u)"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
test -S "$XDG_RUNTIME_DIR/bus"
test "$(podman info --format '{{.Host.CgroupsVersion}}')" = v2
BIN="$ROOT/target/debug/oci_app_manager_lab"
test -x "$BIN"
QD="$HOME/.config/containers/systemd"
SOURCE=p8ci-source
STAGE=p8ci-stage
for slug in "$SOURCE" "$STAGE"; do
  test ! -e "$QD/rh-$slug.container"
  test ! -e "$QD/rh-$slug-data.volume"
  ! podman volume exists "rh-$slug-data"
done
umask 077
WORK="$(mktemp -d "$RUNNER_TEMP/p8ci-private-XXXXXX")"
chmod 0700 "$WORK"
manager() { HARS_DISPOSABLE_OCI_LAB=1 "$BIN" "$@"; }
cleanup_stage() {
  manager --remove-lab "$STAGE" >/dev/null 2>&1 || true
  podman volume rm "rh-$STAGE-data" >/dev/null 2>&1 || true
}
cleanup_all() {
  cleanup_stage
  manager --remove-lab "$SOURCE" >/dev/null 2>&1 || true
  podman volume rm "rh-$SOURCE-data" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup_all EXIT

podman pull --quiet docker.io/getmeili/meilisearch:v1.55.0
podman pull --quiet docker.io/getmeili/meilisearch:v1.54.0
podman pull --quiet docker.io/library/python:3.12-alpine
NEW="$(podman image inspect docker.io/getmeili/meilisearch:v1.55.0 --format '{{index .RepoDigests 0}}')"
OLD="$(podman image inspect docker.io/getmeili/meilisearch:v1.54.0 --format '{{index .RepoDigests 0}}')"
PYTHON="$(podman image inspect docker.io/library/python:3.12-alpine --format '{{index .RepoDigests 0}}')"
[[ "$NEW" == docker.io/getmeili/meilisearch@sha256:* ]]
[[ "$OLD" == docker.io/getmeili/meilisearch@sha256:* ]]
[[ "$PYTHON" == docker.io/library/python@sha256:* ]]
[[ "$NEW" != "$OLD" ]]
install() { manager --install-path-resources-lab "$1" "$2" 1000 /meili_data 1024 256; }
helper() {
  local who="$1" code="$2"
  shift 2
  podman run --rm --network "container:rh-$who" --user 65534 --read-only \
    --cap-drop ALL --security-opt no-new-privileges \
    --pids-limit 32 --memory 96m "$PYTHON" python -B -c "$code" "$@"
}
ready() {
  for _ in $(seq 1 80); do
    if helper "$1" '
import urllib.request
with urllib.request.urlopen("http://127.0.0.1:7700/health",timeout=3) as r:
 assert r.status==200
' >/dev/null 2>&1; then return 0; fi
    sleep .3
  done
  return 1
}
probe() {
  local who="$1" mode="$2"
  helper "$who" '
import json,sys,time,urllib.request
mode=sys.argv[1]; base="http://127.0.0.1:7700"; index="p8ci_versions"
docs=[{"id":1,"title":"amber anchor"},{"id":2,"title":"cerulean comet"}]
def req(method,path,body=None):
 data=None if body is None else json.dumps(body,separators=(",",":")).encode()
 r=urllib.request.Request(base+path,data=data,method=method,
   headers={"Content-Type":"application/json"})
 with urllib.request.urlopen(r,timeout=12) as resp:
  assert resp.status in (200,201,202),resp.status
  return json.load(resp)
version=req("GET","/version")["pkgVersion"]
assert version.startswith("1.55."),version
def await_task(uid):
 for _ in range(130):
  task=req("GET","/tasks/"+str(uid))
  if task["status"]=="succeeded":return
  assert task["status"]!="failed",task
  time.sleep(.25)
 raise TimeoutError("indexing not complete")
if mode=="init":
 await_task(req("POST","/indexes",{"uid":index,"primaryKey":"id"})["taskUid"])
if mode in ("init","after"):
 doc=docs[0] if mode=="init" else docs[1]
 await_task(req("POST","/indexes/"+index+"/documents",[doc])["taskUid"])
expected=docs if mode=="after" else docs[:1]
actual=req("GET","/indexes/"+index+"/documents?limit=10")["results"]
assert sorted(actual,key=lambda x:x["id"])==expected,(mode,actual)
for word,ident in (("anchor",[1]),("comet",[2] if mode=="after" else [])):
 hits=req("GET","/indexes/"+index+"/search?q="+word)["hits"]
 assert [h["id"] for h in hits]==ident,(mode,word,hits)
print("P8CI_REAL_155_"+mode.upper()+"_SEARCH_PASS")
' "$mode"
}
offline() {
  test "$(podman inspect "rh-$1" --format '{{.HostConfig.NetworkMode}}')" = none
  test "$(podman inspect "rh-$1" --format '{{.HostConfig.ReadonlyRootfs}}')" = true
  test "$(podman inspect "rh-$1" --format '{{.HostConfig.Privileged}}')" = false
  test "$(podman inspect "rh-$1" --format '{{.Config.User}}')" = 1000
}
source_ok() {
  ready "$SOURCE"
  offline "$SOURCE"
  probe "$SOURCE" after
  test "$(systemctl --user is-active "rh-$SOURCE.service")" = active
}
install "$SOURCE" "$NEW"
manager --start-lab "$SOURCE"
ready "$SOURCE"
offline "$SOURCE"
probe "$SOURCE" init
manager --stop-lab "$SOURCE"
! podman ps --format '{{.Names}}' | grep -Fx "rh-$SOURCE"
podman volume export --output "$WORK/checkpoint.tar" "rh-$SOURCE-data"
chmod 0600 "$WORK/checkpoint.tar"
test "$(stat -c %a "$WORK/checkpoint.tar")" = 600
tar -tf "$WORK/checkpoint.tar" | grep -Eq '(^|/)data.ms/'
sha256sum "$WORK/checkpoint.tar" > "$WORK/checkpoint.sha256"
sha256sum -c "$WORK/checkpoint.sha256"
echo P8CI_REAL_155_OFFLINE_SHA_CHECKED_PRIVATE_CHECKPOINT_PASS

manager --start-lab "$SOURCE"
ready "$SOURCE"
probe "$SOURCE" after
source_ok
podman volume create "rh-$STAGE-data" >/dev/null
sha256sum -c "$WORK/checkpoint.sha256"
podman volume import "rh-$STAGE-data" "$WORK/checkpoint.tar"
install "$STAGE" "$OLD"
# The valid snapshot is deliberately launched with an older official binary.
# A checksum-valid archive alone does NOT establish application compatibility.
manager --start-lab "$STAGE" > "$WORK/old-start.log" 2>&1 || true
failed=0
for _ in $(seq 1 60); do
  state="$(systemctl --user is-active "rh-$STAGE.service" 2>/dev/null || true)"
  if [[ "$state" == failed || "$state" == inactive ]]; then failed=1; break; fi
  if [[ "$state" == active ]] && helper "$STAGE" '
import urllib.request
with urllib.request.urlopen("http://127.0.0.1:7700/health",timeout=2) as r:
 assert r.status==200
' >/dev/null 2>&1; then
    echo 'INCOMPATIBLE DOWNGRADE STARTED HEALTHY: refusing false pass' >&2
    exit 1
  fi
  sleep .4
done
test "$failed" = 1
journalctl --user -u "rh-$STAGE.service" --no-pager > "$WORK/old-journal.log" 2>&1 || true
if ! grep -Eiq '(database|version).*(incompatib|newer|downgrad|old|mismatch)|(incompatib|newer|downgrad|mismatch).*(database|version)' "$WORK/old-journal.log"; then
  echo 'No version-specific rejection in genuine application logs:' >&2
  tail -55 "$WORK/old-journal.log" >&2
  exit 1
fi
echo P8CI_REAL_154_REFUSES_155_CHECKSUM_VALID_DATABASE_DOWNGRADE_PASS

cleanup_stage
test ! -e "$QD/rh-$STAGE.container"
test ! -e "$QD/rh-$STAGE-data.volume"
! podman volume exists "rh-$STAGE-data"
source_ok
echo P8CI_FAILED_DOWNGRADE_STAGE_REMOVED_ORIGINAL_155_LIVE_PASS

podman volume create "rh-$STAGE-data" >/dev/null
sha256sum -c "$WORK/checkpoint.sha256"
podman volume import "rh-$STAGE-data" "$WORK/checkpoint.tar"
install "$STAGE" "$NEW"
manager --start-lab "$STAGE"
ready "$STAGE"
offline "$STAGE"
probe "$STAGE" restored
source_ok
manager --stop-lab "$STAGE"
manager --start-lab "$STAGE"
ready "$STAGE"
probe "$STAGE" restored
source_ok
echo P8CI_COMPATIBLE_155_RETRY_RESTORES_OLD_INDEX_ORIGINAL_UNCHANGED_PASS

cleanup_stage
test ! -e "$QD/rh-$STAGE.container"
! podman volume exists "rh-$STAGE-data"
source_ok
echo P8CI_RESTORED_APP_REMOVAL_PRESERVES_NEWER_SOURCE_DATA_PASS
