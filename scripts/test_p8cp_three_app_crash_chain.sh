#!/usr/bin/env bash
# P8cp: real three-App rootless dependency chain, unexpected provider SIGKILL.
set -euo pipefail
[[ "${GITHUB_ACTIONS:-}" == true && -n "${RUNNER_TEMP:-}" && "$(id -u)" != 0 ]] || exit 2
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export XDG_RUNTIME_DIR="/run/user/$(id -u)"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
test -S "$XDG_RUNTIME_DIR/bus"
test "$(podman info --format '{{.Host.CgroupsVersion}}')" = v2
BIN="$ROOT/target/debug/oci_app_manager_lab"
test -x "$BIN"
QD="$HOME/.config/containers/systemd"
names=(p8cp-root p8cp-middle p8cp-leaf)
for name in "${names[@]}"; do test ! -e "$QD/rh-$name.container"; done
manager() { HARS_DISPOSABLE_OCI_LAB=1 "$BIN" "$@"; }
cleanup() {
  manager --remove-lab p8cp-leaf >/dev/null 2>&1 || true
  manager --remove-lab p8cp-middle >/dev/null 2>&1 || true
  manager --remove-lab p8cp-root >/dev/null 2>&1 || true
}
trap cleanup EXIT
podman pull --quiet docker.io/library/redis:7.4-alpine
IMAGE="$(podman image inspect docker.io/library/redis:7.4-alpine --format '{{index .RepoDigests 0}}')"
[[ "$IMAGE" == docker.io/library/redis@sha256:* ]]
manager --install-lab p8cp-root "$IMAGE" 999
manager --install-dependent-lab p8cp-middle "$IMAGE" 999 p8cp-root
manager --install-dependent-lab p8cp-leaf "$IMAGE" 999 p8cp-middle
for spec in 'p8cp-middle:p8cp-root' 'p8cp-leaf:p8cp-middle'; do
  child="${spec%%:*}"; parent="${spec##*:}"
  grep -Fx "BindsTo=rh-$parent.service" "$QD/rh-$child.container"
  grep -Fx "After=rh-$parent.service" "$QD/rh-$child.container"
done
for name in "${names[@]}"; do
  test "$(systemctl --user is-active rh-$name.service || true)" = inactive
done
echo P8CP_THREE_REAL_REDIS_CHAIN_NO_AUTOSTART_PASS

manager --start-lab p8cp-leaf
wait_all_ready() {
  for i in $(seq 1 90); do
    ready=true
    for name in "${names[@]}"; do
      if ! podman exec "rh-$name" redis-cli ping 2>/dev/null | grep -qx PONG; then ready=false; fi
    done
    if "$ready"; then return 0; fi
    sleep .5
  done
  return 1
}
wait_all_ready
for name in "${names[@]}"; do
  test "$(systemctl --user is-active rh-$name.service)" = active
  test "$(podman inspect "rh-$name" --format '{{.HostConfig.NetworkMode}}')" = none
  test "$(podman inspect "rh-$name" --format '{{.HostConfig.ReadonlyRootfs}}')" = true
  test "$(podman inspect "rh-$name" --format '{{.Config.User}}')" = 999
  podman exec "rh-$name" redis-cli SET "p8cp-$name" durable | grep -Fx OK
  podman exec "rh-$name" redis-cli SAVE | grep -Fx OK
done
echo P8CP_LEAF_ONLY_STARTS_THREE_ISOLATED_DURABLE_ENGINES_PASS

if manager --remove-lab p8cp-middle; then
  echo 'intermediate provider removed despite dependent' >&2; exit 1
fi
for name in "${names[@]}"; do test "$(systemctl --user is-active rh-$name.service)" = active; done
echo P8CP_INTERMEDIATE_PROVIDER_REMOVAL_REFUSED_PASS

before="$(systemctl --user show -P NRestarts rh-p8cp-root.service)"
[[ "$before" =~ ^[0-9]+$ ]]
podman kill --signal KILL rh-p8cp-root
restarted=false
stopped_children=false
for i in $(seq 1 100); do
  after="$(systemctl --user show -P NRestarts rh-p8cp-root.service)"
  middle="$(systemctl --user is-active rh-p8cp-middle.service || true)"
  leaf="$(systemctl --user is-active rh-p8cp-leaf.service || true)"
  if [[ "$after" =~ ^[0-9]+$ ]] && (( after > before )); then restarted=true; fi
  if [[ "$middle" != active && "$middle" != activating && "$leaf" != active && "$leaf" != activating ]] &&
     ! podman ps --format '{{.Names}}' | grep -Eq '^rh-p8cp-(middle|leaf)$'; then stopped_children=true; fi
  if "$restarted" && "$stopped_children"; then break; fi
  sleep .5
done
"$restarted" || { echo 'root did not auto-restart after SIGKILL' >&2; exit 1; }
"$stopped_children" || {
  systemctl --user status rh-p8cp-root.service rh-p8cp-middle.service rh-p8cp-leaf.service --no-pager || true
  echo 'dependent survived root crash' >&2; exit 1;
}
echo P8CP_UNEXPECTED_ROOT_SIGKILL_CASCADES_TWO_DEPENDENTS_PASS

manager --start-lab p8cp-leaf
wait_all_ready
for name in "${names[@]}"; do
  test "$(systemctl --user is-active rh-$name.service)" = active
  test "$(podman exec "rh-$name" redis-cli GET "p8cp-$name")" = durable
done
echo P8CP_THREE_APP_RECOVERY_PRESERVES_DISTINCT_REDIS_RDB_PASS
cleanup
trap - EXIT
for name in "${names[@]}"; do
  test ! -e "$QD/rh-$name.container"
  podman volume exists "rh-$name-data"
done
echo P8CP_LEAF_FIRST_CLEANUP_PRESERVES_ALL_THREE_PRIVATE_VOLUMES_PASS
