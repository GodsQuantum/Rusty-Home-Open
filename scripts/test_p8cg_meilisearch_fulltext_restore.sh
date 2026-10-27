#!/usr/bin/env bash
# P8cg: ninth distinct real upstream stateful OCI App: Meilisearch full-text index.
# Run ONLY in a disposable, rootless GitHub Actions runner. Never on Cloud9.
set -euo pipefail
[[ "${GITHUB_ACTIONS:-}" == true && -n "${RUNNER_TEMP:-}" && "$(id -u)" != 0 ]] || exit 2
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export XDG_RUNTIME_DIR="/run/user/$(id -u)"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
test -S "$XDG_RUNTIME_DIR/bus"
BIN="$ROOT/target/debug/oci_app_manager_lab"
test -x "$BIN"
QD="$HOME/.config/containers/systemd"
source_slug=fulltext-source
restore_slug=fulltext-restore
source_vol=rh-fulltext-source-data
restore_vol=rh-fulltext-restore-data
test ! -e "$QD/rh-$source_slug.container"
test ! -e "$QD/rh-$restore_slug.container"
! podman volume exists "$source_vol"
! podman volume exists "$restore_vol"
WORK="$(mktemp -d "$RUNNER_TEMP/p8cg-fulltext-XXXXXX")"
chmod 0700 "$WORK"
cleanup() {
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$restore_slug" >/dev/null 2>&1 || true
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$source_slug" >/dev/null 2>&1 || true
  podman volume rm "$restore_vol" "$source_vol" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

# Official upstream v1.55.0, immutable digest only after pull; no image rebuild.
podman pull --quiet docker.io/getmeili/meilisearch:v1.55.0
IMAGE="$(podman image inspect docker.io/getmeili/meilisearch:v1.55.0 --format '{{index .RepoDigests 0}}')"
[[ "$IMAGE" == docker.io/getmeili/meilisearch@sha256:* ]]
podman image exists "$IMAGE"
podman pull --quiet docker.io/library/python:3.12-alpine
PY_IMAGE="$(podman image inspect docker.io/library/python:3.12-alpine --format '{{index .RepoDigests 0}}')"
[[ "$PY_IMAGE" == docker.io/library/python@sha256:* ]]

# Upstream Meilisearch uses /meili_data as its WORKDIR and persistent data root.
# No public port, no external network, no host bind, no secrets, rootless UID.
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-path-resources-lab "$source_slug" "$IMAGE" 1000 /meili_data 1024 256
test "$(systemctl --user is-active rh-$source_slug.service)" = inactive
test "$(stat -c %a "$QD/rh-$source_slug.container")" = 600
grep -Fx 'MemoryMax=1024M' "$QD/rh-$source_slug.container"
grep -Fx 'PidsLimit=256' "$QD/rh-$source_slug.container"
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$source_slug"
assert_offline() {
  test "$(podman inspect "$1" --format '{{.HostConfig.NetworkMode}}')" = none
  test "$(podman inspect "$1" --format '{{.HostConfig.ReadonlyRootfs}}')" = true
  test "$(podman inspect "$1" --format '{{.Config.User}}')" = 1000
  test "$(podman inspect "$1" --format '{{range .Mounts}}{{if eq .Destination "/meili_data"}}{{.Name}}{{end}}{{end}}')" = "$2"
}
helper() {
  local container="$1"
  shift
  # Official read-only Python helper joins only the already Network=none
  # container namespace, talks solely to its own 127.0.0.1:7700.
  podman run --rm --network "container:$container" --user 65534 --read-only \
    --cap-drop ALL --security-opt no-new-privileges --pids-limit 32 \
    --memory 96m "$PY_IMAGE" python -B -c "$1" "${@:2}"
}
ready() {
  for _ in $(seq 1 80); do
    if helper "$1" '
import sys,urllib.request
try:
 with urllib.request.urlopen("http://127.0.0.1:7700/health",timeout=2) as r:
  sys.exit(0 if r.status==200 else 1)
except Exception:sys.exit(1)
' >/dev/null 2>&1; then return 0; fi
    sleep .3
  done
  echo P8CG_MEILISEARCH_UPSTREAM_START_FAILURE_DIAGNOSTICS >&2
  systemctl --user status "$1.service" --no-pager -l >&2 || true
  journalctl --user -u "$1.service" --no-pager -n 90 >&2 || true
  podman logs "$1" 2>&1 | tail -70 >&2 || true
  return 1
}
probe() {
  local container="$1" action="$2"
  helper "$container" '
import json,sys,time,urllib.request,urllib.error
action=sys.argv[1]
base="http://127.0.0.1:7700"
index="p8cg_fulltext_fixture"
first={"id":1,"title":"copper lantern","phase":"before"}
second={"id":2,"title":"violet nebula","phase":"after"}
def req(method,path,body=None):
 data=None if body is None else json.dumps(body,separators=(",",":")).encode()
 r=urllib.request.Request(base+path,method=method,data=data,
                          headers={"Content-Type":"application/json"})
 with urllib.request.urlopen(r,timeout=12) as response:
  assert response.status in (200,201,202),response.status
  return json.load(response)
def await_task(uid):
 end=time.monotonic()+65
 while time.monotonic()<end:
  task=req("GET","/tasks/"+str(uid))
  if task["status"]=="succeeded":return
  if task["status"]=="failed":raise RuntimeError(str(task))
  time.sleep(.3)
 raise TimeoutError("real Meilisearch indexing task did not finish")
def check(expected):
 documents=req("GET","/indexes/"+index+"/documents?limit=10")["results"]
 assert {d["id"] for d in documents}==set(expected),documents
 for d in documents:
  assert d==(first if d["id"]==1 else second),d
 for term,ids in (("lantern",[1]),("nebula",[2] if 2 in expected else [])):
  hits=req("GET","/indexes/"+index+"/search?q="+term)["hits"]
  assert [h["id"] for h in hits]==ids,(term,hits)
if action=="init":
 task=req("POST","/indexes",{"uid":index,"primaryKey":"id"})
 await_task(task["taskUid"])
if action in ("init","after"):
 doc=first if action=="init" else second
 task=req("POST","/indexes/"+index+"/documents",[doc])
 await_task(task["taskUid"])
check([1,2] if action in ("after","both") else [1])
print("P8CG_GENUINE_MEILISEARCH_INDEX_SEARCH_"+action.upper()+"_PASS")
' "$action"
}
ready rh-fulltext-source
assert_offline rh-fulltext-source "$source_vol"
probe rh-fulltext-source init
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --stop-lab "$source_slug"
! podman ps --format '{{.Names}}' | grep -Fx rh-fulltext-source
umask 077
ARCHIVE="$WORK/meilisearch-private.tar"
podman volume export --output "$ARCHIVE" "$source_vol"
chmod 0600 "$ARCHIVE"
test -s "$ARCHIVE"
test "$(stat -c %a "$ARCHIVE")" = 600
tar -tf "$ARCHIVE" > "$WORK/manifest.txt"
grep -Eq '(^|/)data.ms/' "$WORK/manifest.txt"
SHA="$(sha256sum "$ARCHIVE" | cut -d' ' -f1)"
[[ "$SHA" =~ ^[0-9a-f]{64}$ ]]
echo "$SHA  $ARCHIVE" | sha256sum -c
cp "$ARCHIVE" "$WORK/corrupted.tar"
printf CORRUPTED >> "$WORK/corrupted.tar"
if echo "$SHA  $WORK/corrupted.tar" | sha256sum -c >/dev/null 2>&1; then exit 1; fi
echo P8CG_STOPPED_REAL_FULLTEXT_INDEX_PRIVATE_SHA_TAMPER_REFUSED_PASS

HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$source_slug"
ready rh-fulltext-source
probe rh-fulltext-source first
probe rh-fulltext-source after
echo P8CG_SOURCE_INDEXED_SECOND_DOCUMENT_POST_CHECKPOINT_PASS

! podman volume exists "$restore_vol"
podman volume create "$restore_vol" >/dev/null
echo "$SHA  $ARCHIVE" | sha256sum -c
podman volume import "$restore_vol" "$ARCHIVE"
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-path-resources-lab "$restore_slug" "$IMAGE" 1000 /meili_data 1024 256
test "$(systemctl --user is-active rh-$restore_slug.service)" = inactive
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$restore_slug"
ready rh-fulltext-restore
assert_offline rh-fulltext-restore "$restore_vol"
probe rh-fulltext-restore first
probe rh-fulltext-source both
echo P8CG_NINTH_REAL_FULLTEXT_APP_INDEPENDENT_INDEX_RESTORE_POINT_IN_TIME_PASS

HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$restore_slug"
podman volume exists "$restore_vol"
probe rh-fulltext-source both
echo P8CG_RESTORED_SEARCH_APP_REMOVAL_PRESERVES_ORIGINAL_INDEX_PASS
