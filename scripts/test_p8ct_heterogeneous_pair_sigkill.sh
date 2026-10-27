#!/usr/bin/env bash
# P8ct: genuine two DIFFERENT stateful OCI engines with native fsynced
# coordinator intent and SIGKILL recovery, disposable rootless GH Actions only.
set -euo pipefail
[[ "$GITHUB_ACTIONS" == true && -n "$RUNNER_TEMP" && "$(id -u)" != 0 ]] || exit 2
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export XDG_RUNTIME_DIR="/run/user/$(id -u)"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
test -S "$XDG_RUNTIME_DIR/bus"
BIN="$ROOT/target/debug/oci_app_manager_lab"
test -x "$BIN"
QD="$HOME/.config/containers/systemd"
JOURNAL="$QD/.rh-start-pair.journal"
REDIS=p8ct-redis
SEARCH=p8ct-search
manager() { HARS_DISPOSABLE_OCI_LAB=1 "$BIN" "$@"; }
cleanup() {
  if [[ -f "$JOURNAL" ]]; then manager --recover-start-pair-lab >/dev/null 2>&1 || true; fi
  manager --remove-lab "$SEARCH" >/dev/null 2>&1 || true
  manager --remove-lab "$REDIS" >/dev/null 2>&1 || true
}
trap cleanup EXIT
test ! -e "$JOURNAL"
for slug in "$REDIS" "$SEARCH"; do
  test ! -e "$QD/rh-$slug.container"
  test ! -e "$QD/rh-$slug-data.volume"
  ! podman volume exists "rh-$slug-data"
done
podman pull --quiet docker.io/library/redis:7.4-alpine
podman pull --quiet docker.io/getmeili/meilisearch:v1.55.0
podman pull --quiet docker.io/library/python:3.12-alpine
REDIS_IMAGE="$(podman image inspect docker.io/library/redis:7.4-alpine --format '{{index .RepoDigests 0}}')"
SEARCH_IMAGE="$(podman image inspect docker.io/getmeili/meilisearch:v1.55.0 --format '{{index .RepoDigests 0}}')"
PY_IMAGE="$(podman image inspect docker.io/library/python:3.12-alpine --format '{{index .RepoDigests 0}}')"
[[ "$REDIS_IMAGE" == docker.io/library/redis@sha256:* ]]
[[ "$SEARCH_IMAGE" == docker.io/getmeili/meilisearch@sha256:* ]]
[[ "$PY_IMAGE" == docker.io/library/python@sha256:* ]]

# No mutable image tag or path injection; preflight refuses both units.
if manager --install-pair-path-dependent-lab "$REDIS" "$REDIS_IMAGE" /../../etc \
  "$SEARCH" "$SEARCH_IMAGE" /meili_data 999 1000; then exit 1; fi
test ! -e "$QD/rh-$REDIS.container"
test ! -e "$QD/rh-$SEARCH.container"

manager --install-pair-path-dependent-lab "$REDIS" "$REDIS_IMAGE" /data \
  "$SEARCH" "$SEARCH_IMAGE" /meili_data 999 1000
test ! -e "$JOURNAL"
for slug in "$REDIS" "$SEARCH"; do
  test "$(stat -c %a "$QD/rh-$slug.container")" = 600
  test "$(stat -c %a "$QD/rh-$slug-data.volume")" = 600
done
grep -Fx "BindsTo=rh-$REDIS.service" "$QD/rh-$SEARCH.container"
grep -Fx 'MemoryMax=1024M' "$QD/rh-$SEARCH.container"
test "$(systemctl --user is-active rh-$REDIS.service)" = inactive
test "$(systemctl --user is-active rh-$SEARCH.service)" = inactive
echo P8CT_ATOMIC_TWO_DIFFERENT_NATIVE_APP_DESCRIPTORS_OFFLINE_INERT_PASS

meili_helper() {
  local app="$1" op="$2"
  podman run --rm --network "container:rh-$app" --user 65534 --read-only \
    --cap-drop ALL --security-opt no-new-privileges --pids-limit 32 \
    --memory 96m "$PY_IMAGE" python -B -c '
import json,sys,time,urllib.request
op=sys.argv[1]
base="http://127.0.0.1:7700"
def req(method,path,body=None):
 data=None if body is None else json.dumps(body).encode()
 r=urllib.request.Request(base+path,data=data,method=method,headers={"Content-Type":"application/json"})
 with urllib.request.urlopen(r,timeout=8) as response:
  assert response.status in (200,201,202)
  return json.load(response)
def settle(task):
 end=time.monotonic()+40
 while time.monotonic()<end:
  state=req("GET","/tasks/"+str(task))
  if state["status"]=="succeeded":return
  if state["status"]=="failed":raise AssertionError("Meilisearch index failed")
  time.sleep(.3)
 raise TimeoutError("Meilisearch task did not complete")
if op=="ready":
 assert req("GET","/health")["status"]=="available"
 sys.exit(0)
if op=="init":
 settle(req("POST","/indexes",{"uid":"p8ct_native","primaryKey":"id"})["taskUid"])
 settle(req("POST","/indexes/p8ct_native/documents",[{"id":1,"title":"durable cedar lantern"}])["taskUid"])
docs=req("GET","/indexes/p8ct_native/documents?limit=10")["results"]
assert docs==[{"id":1,"title":"durable cedar lantern"}],docs
hits=req("GET","/indexes/p8ct_native/search?q=lantern")["hits"]
assert [h["id"] for h in hits]==[1],hits
' "$op"
}
ready_search() {
  for _ in $(seq 1 80); do
    if meili_helper "$SEARCH" ready >/dev/null 2>&1; then return 0; fi
    sleep .4
  done
  systemctl --user status "rh-$SEARCH.service" --no-pager >&2 || true
  podman logs "rh-$SEARCH" >&2 || true
  return 1
}
assert_offline() {
  local slug="$1" uid="$2" mount="$3"
  test "$(podman inspect "rh-$slug" --format '{{.HostConfig.NetworkMode}}')" = none
  test "$(podman inspect "rh-$slug" --format '{{.HostConfig.ReadonlyRootfs}}')" = true
  test "$(podman inspect "rh-$slug" --format '{{.HostConfig.Privileged}}')" = false
  test "$(podman inspect "rh-$slug" --format '{{.Config.User}}')" = "$uid"
  test "$(podman inspect "rh-$slug" --format "{{range .Mounts}}{{if eq .Destination \"$mount\"}}{{.Name}}{{end}}{{end}}")" = "rh-$slug-data"
}
manager --start-pair-transaction-lab "$REDIS" "$SEARCH"
test ! -e "$JOURNAL"
ready_search
assert_offline "$REDIS" 999 /data
assert_offline "$SEARCH" 1000 /meili_data
test "$(podman exec rh-$REDIS redis-cli PING)" = PONG
podman exec rh-$REDIS redis-cli SET durable-key saved-redis | grep -Fx OK
saved="$(podman exec rh-$REDIS redis-cli SAVE 2>&1)" || {
  echo "P8CT_REDIS_SAVE_EXEC_FAILED: $saved" >&2
  exit 1
}
if [[ "$saved" != OK ]]; then
  echo "P8CT_REDIS_SAVE_REJECTED: $saved" >&2
  podman logs "rh-$REDIS" 2>&1 | tail -35 >&2 || true
  exit 1
fi
echo P8CT_REDIS_DISK_SAVE_CONFIRMED_PASS
meili_helper "$SEARCH" init
echo P8CT_TRUE_REDIS_PLUS_MEILISEARCH_BOTH_STATEFUL_ENGINES_DURABLE_DATA_PASS

manager --stop-lab "$SEARCH"
manager --stop-lab "$REDIS"
if HARS_P8CS_KILL_AFTER_PROVIDER=1 manager --start-pair-transaction-lab "$REDIS" "$SEARCH"; then
  echo 'coordinator SIGKILL failed to interrupt heterogenous pair' >&2
  exit 1
fi
test -f "$JOURNAL"
test "$(stat -c %a "$JOURNAL")" = 600
grep -Fx P8CS1 "$JOURNAL"
grep -Fx "$REDIS" "$JOURNAL"
grep -Fx "$SEARCH" "$JOURNAL"
test "$(systemctl --user is-active rh-$REDIS.service)" = active
! systemctl --user is-active --quiet "rh-$SEARCH.service"
if manager --start-lab "$SEARCH" || manager --remove-lab "$REDIS"; then
  echo 'blocked concurrent mutator bypassed durable pending-intent' >&2
  exit 1
fi
echo P8CT_NATIVE_COORDINATOR_SIGKILL_BETWEEN_DIFFERENT_ENGINES_FENCED_PASS

# This is a separate process; stop ONLY, never deletes the database volumes.
manager --recover-start-pair-lab
test ! -e "$JOURNAL"
! systemctl --user is-active --quiet "rh-$REDIS.service"
! systemctl --user is-active --quiet "rh-$SEARCH.service"
for slug in "$REDIS" "$SEARCH"; do podman volume exists "rh-$slug-data"; done
echo P8CT_DIFFERENT_APP_PARTIAL_START_CRASH_STOP_ONLY_RECOVERY_PASS
manager --start-pair-transaction-lab "$REDIS" "$SEARCH"
ready_search
test "$(podman exec rh-$REDIS redis-cli GET durable-key)" = saved-redis
meili_helper "$SEARCH" check
echo P8CT_DIFFERENT_REDIS_RDB_AND_MEILI_INDEX_SURVIVE_COORDINATOR_RESTART_PASS
manager --remove-lab "$SEARCH"
manager --remove-lab "$REDIS"
for slug in "$REDIS" "$SEARCH"; do podman volume exists "rh-$slug-data"; done
echo P8CT_ORDERED_NATIVE_DIFFERENT_APP_CLEANUP_PRESERVES_BOTH_DATABASES_PASS
