#!/usr/bin/env bash
# Fifth DISTINCT App, genuine official Node-RED HTTP Admin API, persisted flows.
# Disposable rootless GitHub CI only. Never Cloud9/HAOS household.
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
source_slug=red-flow-source
restore_slug=red-flow-restore
source_vol=rh-red-flow-source-data
restore_vol=rh-red-flow-restore-data
test ! -e "$QD/rh-$source_slug.container"
test ! -e "$QD/rh-$restore_slug.container"
! podman volume exists "$source_vol"
! podman volume exists "$restore_vol"
WORK="$(mktemp -d "$(printenv RUNNER_TEMP)/p8cc-nodered-XXXXXX")"
chmod 0700 "$WORK"
cleanup() {
 HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$restore_slug" >/dev/null 2>&1 || true
 HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$source_slug" >/dev/null 2>&1 || true
 podman volume rm "$restore_vol" "$source_vol" >/dev/null 2>&1 || true
 rm -rf "$WORK"
}
trap cleanup EXIT
podman pull --quiet docker.io/nodered/node-red:latest
IMAGE="$(podman image inspect docker.io/nodered/node-red:latest --format '{{index .RepoDigests 0}}')"
[[ "$IMAGE" == docker.io/nodered/node-red@sha256:* ]]
podman image exists "$IMAGE"

# Generic upstream ENTRYPOINT, no custom entrypoint or host HTTP port.
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-lab "$source_slug" "$IMAGE" 1000
test "$(systemctl --user is-active rh-$source_slug.service)" = inactive
test "$(stat -c %a "$QD/rh-$source_slug.container")" = 600
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$source_slug"
assert_offline() {
 test "$(podman inspect "$1" --format '{{.HostConfig.NetworkMode}}')" = none
 test "$(podman inspect "$1" --format '{{.HostConfig.ReadonlyRootfs}}')" = true
 test "$(podman inspect "$1" --format '{{.Config.User}}')" = 1000
 test "$(podman inspect "$1" --format '{{range .Mounts}}{{if eq .Destination "/data"}}{{.Name}}{{end}}{{end}}')" = "$2"
}
flow_api() {
 podman exec "$1" node -e '
(async () => {
 const op = process.argv[1], expected = process.argv[2];
 const url="http://127.0.0.1:1880/flows";
 const headers={"Node-RED-API-Version":"v1","Content-Type":"application/json"};
 const flows=[{"id":"a111a111a111a111","type":"tab","label":"before-checkpoint","disabled":false,"info":""}];
 if (expected==="both") flows.push({"id":"b222b222b222b222","type":"tab","label":"after-checkpoint","disabled":false,"info":""});
 if (op==="post") {
  const response=await fetch(url,{method:"POST",headers,body:JSON.stringify(flows)});
  if (response.status!==204) throw new Error("POST status "+response.status+" "+(await response.text()).slice(0,180));
 }
 const response=await fetch(url,{headers});
 if (!response.ok) throw new Error("GET status "+response.status);
 const found=(await response.json()).map(item=>item.label).sort();
 const want=expected==="both" ? ["after-checkpoint","before-checkpoint"] : ["before-checkpoint"];
 if (JSON.stringify(found)!==JSON.stringify(want)) throw new Error("Persisted flow mismatch "+JSON.stringify(found));
 console.log("P8CC_REAL_NODE_RED_HTTP_API_"+op.toUpperCase()+"_"+expected.toUpperCase()+"_PASS");
})().catch(e=>{console.error(e.message);process.exit(1)});
' "$2" "$3"
}
ready() {
 for _ in $(seq 1 100); do
  if podman exec "$1" node -e '
fetch("http://127.0.0.1:1880/flows",{headers:{"Node-RED-API-Version":"v1"}})
.then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))' >/dev/null 2>&1; then return 0; fi
  sleep 0.2
 done
 return 1
}
ready rh-red-flow-source
assert_offline rh-red-flow-source "$source_vol"
flow_api rh-red-flow-source post first
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --stop-lab "$source_slug"
! podman ps --format '{{.Names}}' | grep -Fx rh-red-flow-source
umask 077
ARCHIVE="$WORK/nodered-private.tar"
podman volume export --output "$ARCHIVE" "$source_vol"
chmod 0600 "$ARCHIVE"
test "$(stat -c %a "$ARCHIVE")" = 600
test -s "$ARCHIVE"
tar -tf "$ARCHIVE" > "$WORK/archive-files.txt"
grep -Eq '(^|/)flows.json$' "$WORK/archive-files.txt"
SHA="$(sha256sum "$ARCHIVE" | cut -d' ' -f1)"
[[ "$SHA" =~ ^[0-9a-f]{64}$ ]]
echo "$SHA  $ARCHIVE" | sha256sum -c
cp "$ARCHIVE" "$WORK/bad.tar"
printf CORRUPTED >> "$WORK/bad.tar"
if echo "$SHA  $WORK/bad.tar" | sha256sum -c >/dev/null 2>&1; then exit 1; fi
echo P8CC_OFFICIAL_NODE_RED_STOPPED_FLOW_VOLUME_SHA_TAMPER_REFUSAL_PASS

HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$source_slug"
ready rh-red-flow-source
flow_api rh-red-flow-source get first
flow_api rh-red-flow-source post both
echo P8CC_OFFICIAL_NODE_RED_POST_CHECKPOINT_SECOND_REAL_FLOW_CREATED_PASS

! podman volume exists "$restore_vol"
podman volume create "$restore_vol" >/dev/null
echo "$SHA  $ARCHIVE" | sha256sum -c
podman volume import "$restore_vol" "$ARCHIVE"
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-lab "$restore_slug" "$IMAGE" 1000
test "$(systemctl --user is-active rh-$restore_slug.service)" = inactive
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$restore_slug"
ready rh-red-flow-restore
assert_offline rh-red-flow-restore "$restore_vol"
flow_api rh-red-flow-restore get first
flow_api rh-red-flow-source get both
echo P8CC_FIFTH_REAL_AUTOMATION_APP_NATIVE_FLOW_INDEPENDENT_RESTORE_PASS
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$restore_slug"
podman volume exists "$restore_vol"
flow_api rh-red-flow-source get both
echo P8CC_NODE_RED_RESTORED_APP_REMOVE_PRESERVES_SOURCE_AND_SNAPSHOT_PASS
