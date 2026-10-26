#!/usr/bin/env bash
# Disposable GitHub runner only. Real official Redis OCI image + systemd --user.
set -euo pipefail
[[ "${GITHUB_ACTIONS:-}" == true && -n "${RUNNER_TEMP:-}" && "$(id -u)" != 0 ]] || exit 2
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}"
test -S "$XDG_RUNTIME_DIR/bus"
systemctl --user show-environment >/dev/null
podman info --format '{{.Host.CgroupsVersion}}' | grep -Fx v2
podman pull --quiet docker.io/library/redis:7.4-alpine
SOURCE="$(podman image inspect docker.io/library/redis:7.4-alpine --format '{{index .RepoDigests 0}}')"
[[ "$SOURCE" == docker.io/library/redis@sha256:* ]]
podman image exists "$SOURCE"
BIN="$ROOT/target/debug/oci_quadlet_lab"
test -x "$BIN"
APP_DIR="$(mktemp -d "$RUNNER_TEMP/oci-live-XXXXXX")"
chmod 0700 "$APP_DIR"
QUADLETS="$HOME/.config/containers/systemd"
mkdir -p "$QUADLETS"
[[ ! -e "$QUADLETS/rh-redis.container" && ! -e "$QUADLETS/rh-redis-data.volume" ]] || exit 2
cleanup() {
  systemctl --user stop rh-redis.service >/dev/null 2>&1 || true
  rm -f "$QUADLETS/rh-redis.container" "$QUADLETS/rh-redis-data.volume"
  systemctl --user daemon-reload >/dev/null 2>&1 || true
}
trap cleanup EXIT
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --generate-lab redis "$SOURCE" "$APP_DIR" --uid 999
install -m 0600 "$APP_DIR/rh-redis.container" "$QUADLETS/rh-redis.container"
install -m 0600 "$APP_DIR/rh-redis-data.volume" "$QUADLETS/rh-redis-data.volume"
systemctl --user daemon-reload
systemctl --user cat rh-redis.service >/dev/null
systemctl --user start rh-redis.service
for i in $(seq 1 90); do
  if podman exec rh-redis redis-cli ping 2>/dev/null | grep -Fx PONG; then break; fi
  sleep .5
done
podman exec rh-redis redis-cli ping | grep -Fx PONG
test "$(podman inspect rh-redis --format '{{.HostConfig.NetworkMode}}')" = none
test "$(podman inspect rh-redis --format '{{.HostConfig.ReadonlyRootfs}}')" = true
podman exec rh-redis redis-cli SET rusty-lab-proof 'persistent-data' | grep -Fx OK
podman exec rh-redis redis-cli SAVE | grep -Fx OK
systemctl --user stop rh-redis.service
! podman ps --format '{{.Names}}' | grep -Fx rh-redis
systemctl --user start rh-redis.service
for i in $(seq 1 90); do
  if podman exec rh-redis redis-cli ping 2>/dev/null | grep -Fx PONG; then break; fi
  sleep .5
done
test "$(podman exec rh-redis redis-cli GET rusty-lab-proof)" = persistent-data
echo P8AP_REAL_ROOTLESS_REDIS_QUADLET_START_STOP_PERSISTENT_DATA_PASS
