#!/usr/bin/env bash
# P8cq: native Podman Notify=healthy fences dependent startup and live health loss.
# Runs ONLY on a disposable nonroot GitHub Actions runner, never on Cloud9.
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
for name in p8cq-provider p8cq-child p8cq-poison; do test ! -e "$QD/rh-$name.container"; done
manager() { HARS_DISPOSABLE_OCI_LAB=1 "$BIN" "$@"; }
cleanup() {
  manager --remove-lab p8cq-child >/dev/null 2>&1 || true
  manager --remove-lab p8cq-provider >/dev/null 2>&1 || true
}
trap cleanup EXIT
podman pull --quiet docker.io/library/redis:7.4-alpine
IMAGE="$(podman image inspect docker.io/library/redis:7.4-alpine --format '{{index .RepoDigests 0}}')"
[[ "$IMAGE" == docker.io/library/redis@sha256:* ]]

# A real upstream Redis process starts, but its deliberately wrong localhost
# port makes its native Podman healthcheck fail. systemd must NOT mark ready.
manager --install-health-ready-lab p8cq-provider "$IMAGE" 999 'redis-cli -p 6399 ping'
manager --install-dependent-lab p8cq-child "$IMAGE" 999 p8cq-provider
grep -Fx 'Notify=healthy' "$QD/rh-p8cq-provider.container"
grep -Fx 'BindsTo=rh-p8cq-provider.service' "$QD/rh-p8cq-child.container"
manager --start-lab p8cq-child >"$RUNNER_TEMP/p8cq-negative-start.log" 2>&1 &
start_pid=$!
observed=false
for i in $(seq 1 80); do
  if podman inspect rh-p8cq-provider --format '{{.State.Running}}' 2>/dev/null | grep -qx true; then
    observed=true
    break
  fi
  sleep .5
done
"$observed" || { cat "$RUNNER_TEMP/p8cq-negative-start.log"; exit 1; }
sleep 3
test "$(systemctl --user is-active rh-p8cq-provider.service || true)" = activating
test "$(systemctl --user is-active rh-p8cq-child.service || true)" != active
! podman container exists rh-p8cq-child
! podman healthcheck run rh-p8cq-provider >/dev/null 2>&1
echo P8CQ_REAL_RUNNING_BUT_UNHEALTHY_PROVIDER_BLOCKS_DEPENDENT_START_PASS
# Cancel ONLY the disposable provider's pending start, never the real host.
systemctl --user stop rh-p8cq-provider.service
if wait "$start_pid"; then
  echo 'dependent unexpectedly started without provider readiness' >&2; exit 1
fi
cleanup
test ! -e "$QD/rh-p8cq-provider.container"
test ! -e "$QD/rh-p8cq-child.container"
echo P8CQ_FAILED_HEALTH_START_CLEANUP_NO_DANGLING_UNITS_PASS

# Same genuine OCI engine, this time a working application-level probe.
manager --install-health-ready-lab p8cq-provider "$IMAGE" 999 'redis-cli ping'
manager --install-health-dependent-ready-lab p8cq-child "$IMAGE" 999 'redis-cli ping' p8cq-provider
if manager --install-health-dependent-ready-lab p8cq-poison "$IMAGE" 999 'redis-cli; id' p8cq-provider; then
  echo 'unsafe health command accepted' >&2; exit 1
fi
test ! -e "$QD/rh-p8cq-poison.container"
grep -Fx 'Notify=healthy' "$QD/rh-p8cq-provider.container"
grep -Fx 'Notify=healthy' "$QD/rh-p8cq-child.container"
manager --start-lab p8cq-child
for name in p8cq-provider p8cq-child; do
  test "$(systemctl --user is-active rh-$name.service)" = active
  test "$(podman inspect rh-$name --format '{{.State.Health.Status}}')" = healthy
  test "$(podman inspect rh-$name --format '{{.HostConfig.NetworkMode}}')" = none
  test "$(podman inspect rh-$name --format '{{.HostConfig.ReadonlyRootfs}}')" = true
  test "$(podman inspect rh-$name --format '{{.Config.User}}')" = 999
  podman exec rh-$name redis-cli ping | grep -Fx PONG
  podman exec rh-$name redis-cli SET "p8cq-$name" durable | grep -Fx OK
  podman exec rh-$name redis-cli SAVE | grep -Fx OK
done
provider_time="$(systemctl --user show -P ActiveEnterTimestampMonotonic rh-p8cq-provider.service)"
child_time="$(systemctl --user show -P ActiveEnterTimestampMonotonic rh-p8cq-child.service)"
[[ "$provider_time" =~ ^[0-9]+$ && "$child_time" =~ ^[0-9]+$ ]]
(( provider_time > 0 && child_time >= provider_time ))
echo P8CQ_TWO_REAL_REDIS_APPS_WAIT_FOR_NATIVE_HEALTH_READY_PASS

# Induce APPLICATION-LEVEL failure without SIGKILL: Redis PID stays running
# while CLIENT PAUSE makes genuine redis-cli health probes time out.
before="$(systemctl --user show -P NRestarts rh-p8cq-provider.service)"
[[ "$before" =~ ^[0-9]+$ ]]
podman exec rh-p8cq-provider redis-cli CLIENT PAUSE 60000 ALL | grep -Fx OK
# Immediately verify Redis container process is still alive after the pause.
test "$(podman inspect rh-p8cq-provider --format '{{.State.Running}}')" = true
restarted=false
fenced=false
for i in $(seq 1 170); do
  after="$(systemctl --user show -P NRestarts rh-p8cq-provider.service)"
  [[ "$after" =~ ^[0-9]+$ ]]
  if (( after > before )); then restarted=true; fi
  state="$(systemctl --user is-active rh-p8cq-child.service || true)"
  if [[ "$state" != active && "$state" != activating ]] &&
     ! podman ps --format '{{.Names}}' | grep -qx rh-p8cq-child; then fenced=true; fi
  if "$restarted" && "$fenced"; then break; fi
  sleep .5
done
"$restarted" && "$fenced" || {
  systemctl --user status rh-p8cq-provider.service rh-p8cq-child.service --no-pager || true
  podman inspect rh-p8cq-provider --format '{{json .State.Health}}' || true
  echo 'health failure did not restart provider and fence dependent' >&2; exit 1
}
echo P8CQ_REAL_LIVE_REDIS_HEALTH_FAILURE_KILLS_PROVIDER_AND_FENCES_CHILD_PASS
manager --start-lab p8cq-child
for name in p8cq-provider p8cq-child; do
  test "$(systemctl --user is-active rh-$name.service)" = active
  test "$(podman inspect rh-$name --format '{{.State.Health.Status}}')" = healthy
  test "$(podman exec rh-$name redis-cli GET "p8cq-$name")" = durable
done
echo P8CQ_NATIVE_HEALTH_RECOVERY_PRESERVES_TWO_PRIVATE_RDB_KEYS_PASS
cleanup
trap - EXIT
for name in p8cq-provider p8cq-child; do
  test ! -e "$QD/rh-$name.container"
  podman volume exists "rh-$name-data"
done
echo P8CQ_DEPENDENT_FIRST_CLEANUP_PRESERVES_PRIVATE_DATA_PASS
