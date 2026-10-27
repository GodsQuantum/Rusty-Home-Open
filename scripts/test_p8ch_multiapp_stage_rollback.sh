#!/usr/bin/env bash
# P8ch: real Redis + Meilisearch; fail the second checkpoint after first App
# restoration, roll back the partial stage, retry both. GitHub rootless only.
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
RP=tx-redis-primary; MP=tx-meili-primary
RS=tx-redis-stage; MS=tx-meili-stage
for app in "$RP" "$MP" "$RS" "$MS"; do
  test ! -e "$QD/rh-$app.container"
  test ! -e "$QD/rh-$app-data.volume"
  ! podman volume exists "rh-$app-data"
done
umask 077
WORK="$(mktemp -d "$RUNNER_TEMP/p8ch-private-XXXXXX")"
chmod 0700 "$WORK"
manager() { HARS_DISPOSABLE_OCI_LAB=1 "$BIN" "$@"; }
cleanup_stage() {
  manager --remove-lab "$MS" >/dev/null 2>&1 || true
  manager --remove-lab "$RS" >/dev/null 2>&1 || true
  podman volume rm "rh-$MS-data" "rh-$RS-data" >/dev/null 2>&1 || true
}
cleanup_all() {
  cleanup_stage
  manager --remove-lab "$MP" >/dev/null 2>&1 || true
  manager --remove-lab "$RP" >/dev/null 2>&1 || true
  podman volume rm "rh-$MP-data" "rh-$RP-data" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup_all EXIT

podman pull --quiet docker.io/library/redis:7.4-alpine
podman pull --quiet docker.io/getmeili/meilisearch:v1.55.0
podman pull --quiet docker.io/library/python:3.12-alpine
REDIS="$(podman image inspect docker.io/library/redis:7.4-alpine --format '{{index .RepoDigests 0}}')"
MEILI="$(podman image inspect docker.io/getmeili/meilisearch:v1.55.0 --format '{{index .RepoDigests 0}}')"
PYTHON="$(podman image inspect docker.io/library/python:3.12-alpine --format '{{index .RepoDigests 0}}')"
[[ "$REDIS" == docker.io/library/redis@sha256:* ]]
[[ "$MEILI" == docker.io/getmeili/meilisearch@sha256:* ]]
[[ "$PYTHON" == docker.io/library/python@sha256:* ]]
[[ "$REDIS" != "$MEILI" ]]
redis_install() { manager --install-lab "$1" "$REDIS" 999; }
meili_install() { manager --install-path-resources-lab "$1" "$MEILI" 1000 /meili_data 1024 256; }
redis_ready() {
  for _ in $(seq 1 60); do
    if podman exec "rh-$1" redis-cli ping 2>/dev/null | grep -Fxq PONG; then return 0; fi
    sleep .3
  done
  return 1
}
meili_helper() {
  local who="$1" code="$2"
  shift 2
  podman run --rm --network "container:rh-$who" --user 65534 --read-only \
    --cap-drop ALL --security-opt no-new-privileges \
    --pids-limit 32 --memory 96m "$PYTHON" python -B -c "$code" "$@"
}
meili_probe() {
  local who="$1" op="$2"
  meili_helper "$who" '
import json,sys,time,urllib.request
action=sys.argv[1]; base="http://127.0.0.1:7700"
index="p8ch_restore"; docs=({"id":1,"title":"amber anchor","version":"before"},
                           {"id":2,"title":"cerulean comet","version":"after"})
def req(method,path,body=None):
 data=None if body is None else json.dumps(body,separators=(",",":")).encode()
 request=urllib.request.Request(base+path,data=data,method=method,
  headers={"Content-Type":"application/json"})
 with urllib.request.urlopen(request,timeout=12) as response:
  assert response.status in (200,201,202),response.status
  return json.load(response)
def await_task(uid):
 for _ in range(130):
  task=req("GET","/tasks/"+str(uid))
  if task["status"]=="succeeded":return
  assert task["status"]!="failed",task
  time.sleep(.25)
 raise TimeoutError("Meilisearch indexing task unfinished")
if action=="init":
 await_task(req("POST","/indexes",{"uid":index,"primaryKey":"id"})["taskUid"])
if action in ("init","after"):
 doc=docs[0] if action=="init" else docs[1]
 await_task(req("POST","/indexes/"+index+"/documents",[doc])["taskUid"])
expected=docs if action in ("after","both") else docs[:1]
actual=req("GET","/indexes/"+index+"/documents?limit=10")["results"]
assert sorted(actual,key=lambda x:x["id"])==list(expected),(action,actual)
for word,ident in (("anchor",[1]),("comet",[2] if len(expected)==2 else [])):
 hits=req("GET","/indexes/"+index+"/search?q="+word)["hits"]
 assert [h["id"] for h in hits]==ident,(action,word,hits)
print("P8CH_MEILI_"+action.upper()+"_PASS")
' "$op"
}
meili_ready() {
  for _ in $(seq 1 80); do
    if meili_helper "$1" '
import urllib.request
with urllib.request.urlopen("http://127.0.0.1:7700/health",timeout=3) as r:
 assert r.status==200
' >/dev/null 2>&1; then return 0; fi
    sleep .3
  done
  systemctl --user status "rh-$1.service" --no-pager >&2 || true
  return 1
}
offline() {
  test "$(podman inspect "rh-$1" --format '{{.HostConfig.NetworkMode}}')" = none
  test "$(podman inspect "rh-$1" --format '{{.HostConfig.ReadonlyRootfs}}')" = true
  test "$(podman inspect "rh-$1" --format '{{.HostConfig.Privileged}}')" = false
  test "$(podman inspect "rh-$1" --format '{{.Config.User}}')" = "$2"
}
source_ok() {
  redis_ready "$RP"; meili_ready "$MP"
  offline "$RP" 999; offline "$MP" 1000
  test "$(podman exec "rh-$RP" redis-cli GET p8ch-before)" = first
  test "$(podman exec "rh-$RP" redis-cli GET p8ch-after)" = second
  meili_probe "$MP" both
}
source_only() {
  test ! -e "$QD/rh-$RS.container"
  test ! -e "$QD/rh-$MS.container"
  ! podman volume exists "rh-$RS-data"
  ! podman volume exists "rh-$MS-data"
  test "$(systemctl --user is-active "rh-$RP.service")" = active
  test "$(systemctl --user is-active "rh-$MP.service")" = active
}
redis_install "$RP"; meili_install "$MP"
manager --start-lab "$RP"; manager --start-lab "$MP"
redis_ready "$RP"; meili_ready "$MP"
offline "$RP" 999; offline "$MP" 1000
podman exec "rh-$RP" redis-cli SET p8ch-before first | grep -Fx OK
podman exec "rh-$RP" redis-cli SAVE | grep -Fx OK
meili_probe "$MP" init

# The 2 writers are quiesced before the group checkpoint; both immutable
# archives are private and checksum pinned. This is NOT atomic live backup.
manager --stop-lab "$MP"; manager --stop-lab "$RP"
for slug in "$RP" "$MP"; do
  ! podman ps --format '{{.Names}}' | grep -Fx "rh-$slug"
  podman volume export --output "$WORK/$slug.tar" "rh-$slug-data"
  chmod 0600 "$WORK/$slug.tar"
  test -s "$WORK/$slug.tar"
  test "$(stat -c %a "$WORK/$slug.tar")" = 600
  sha256sum "$WORK/$slug.tar" > "$WORK/$slug.sha256"
  sha256sum -c "$WORK/$slug.sha256"
done
tar -tf "$WORK/$RP.tar" | grep -Eq '(^|/)dump.rdb$'
tar -tf "$WORK/$MP.tar" | grep -Eq '(^|/)data.ms/'
echo P8CH_TWO_STATEFUL_APPS_SAME_QUIESCED_CHECKPOINT_PASS

manager --start-lab "$RP"; manager --start-lab "$MP"
redis_ready "$RP"; meili_ready "$MP"
podman exec "rh-$RP" redis-cli SET p8ch-after second | grep -Fx OK
podman exec "rh-$RP" redis-cli SAVE | grep -Fx OK
meili_probe "$MP" after
source_ok
echo P8CH_BOTH_ORIGINAL_APPS_POST_CHECKPOINT_MUTATIONS_PASS

# Partial restore: first REAL App is running; now second archive is tampered.
# The second app must never import corrupted data or start. Roll back first.
cp "$WORK/$MP.tar" "$WORK/invalid-meili.tar"
printf INJECTED_FAULT >> "$WORK/invalid-meili.tar"
podman volume create "rh-$RS-data" >/dev/null
sha256sum -c "$WORK/$RP.sha256"
podman volume import "rh-$RS-data" "$WORK/$RP.tar"
redis_install "$RS"
manager --start-lab "$RS"
redis_ready "$RS"
offline "$RS" 999
test "$(podman exec "rh-$RS" redis-cli GET p8ch-before)" = first
test -z "$(podman exec "rh-$RS" redis-cli GET p8ch-after)"
if echo "$(cut -d' ' -f1 "$WORK/$MP.sha256")  $WORK/invalid-meili.tar" | sha256sum -c >/dev/null 2>&1; then
  echo 'Corrupted second archive was accepted' >&2
  exit 1
fi
test ! -e "$QD/rh-$MS.container"
echo P8CH_SECOND_ARCHIVE_TAMPER_REJECTED_AFTER_FIRST_RESTORE_STARTED_PASS
cleanup_stage
source_only
source_ok
echo P8CH_FAILED_PARTIAL_BATCH_CLEANUP_ORIGINALS_PRESERVED_PASS

# Retry both real independent volumes. Promotion into the household is NOT
# attempted: this proves only isolated staged recovery and failure cleanup.
for slug in "$RS" "$MS"; do
  ! podman volume exists "rh-$slug-data"
  podman volume create "rh-$slug-data" >/dev/null
done
sha256sum -c "$WORK/$RP.sha256"
sha256sum -c "$WORK/$MP.sha256"
podman volume import "rh-$RS-data" "$WORK/$RP.tar"
podman volume import "rh-$MS-data" "$WORK/$MP.tar"
redis_install "$RS"; meili_install "$MS"
manager --start-lab "$RS"; manager --start-lab "$MS"
redis_ready "$RS"; meili_ready "$MS"
offline "$RS" 999; offline "$MS" 1000
test "$(podman exec "rh-$RS" redis-cli GET p8ch-before)" = first
test -z "$(podman exec "rh-$RS" redis-cli GET p8ch-after)"
meili_probe "$MS" first
source_ok
echo P8CH_DIVERSE_PAIR_INDEPENDENT_RECOVERY_SHARED_POINT_IN_TIME_PASS

manager --stop-lab "$MS"; manager --stop-lab "$RS"
manager --start-lab "$RS"; manager --start-lab "$MS"
redis_ready "$RS"; meili_ready "$MS"
test "$(podman exec "rh-$RS" redis-cli GET p8ch-before)" = first
test -z "$(podman exec "rh-$RS" redis-cli GET p8ch-after)"
meili_probe "$MS" first
source_ok
echo P8CH_RESTORED_PAIR_RESTART_DURABLE_SOURCES_UNAFFECTED_PASS
cleanup_stage
source_only
source_ok
echo P8CH_TWO_ORIGINALS_UNTOUCHED_AFTER_STAGE_REMOVAL_PASS
