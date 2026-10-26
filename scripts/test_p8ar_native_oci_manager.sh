#!/usr/bin/env bash
set -euo pipefail
[[ "$(printenv GITHUB_ACTIONS || true)" == true && -n "$(printenv RUNNER_TEMP || true)" && "$(id -u)" != 0 ]] || exit 2
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export XDG_RUNTIME_DIR="/run/user/$(id -u)"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
test -S "$XDG_RUNTIME_DIR/bus"
podman info --format '{{.Host.CgroupsVersion}}' | grep -Fx v2
podman pull --quiet docker.io/library/redis:7.4-alpine
SOURCE="$(podman image inspect docker.io/library/redis:7.4-alpine --format '{{index .RepoDigests 0}}')"
[[ "$SOURCE" == docker.io/library/redis@sha256:* ]]
BIN="$ROOT/target/debug/oci_app_manager_lab"
test -x "$BIN"
QUADLETS="$HOME/.config/containers/systemd"
test ! -e "$QUADLETS/rh-managedredis.container"
cleanup() {
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab managedredis >/dev/null 2>&1 || true
}
trap cleanup EXIT

# Install is declared but must not start or enable the unit automatically.
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-lab managedredis "$SOURCE" 999
test -f "$QUADLETS/rh-managedredis.container"
test -f "$QUADLETS/rh-managedredis-data.volume"
test "$(stat -c '%a' "$QUADLETS/rh-managedredis.container")" = 600
state="$(systemctl --user is-enabled rh-managedredis.service)"
[[ "$state" == static || "$state" == generated ]]
[[ "$state" != enabled ]]
[[ "$(systemctl --user is-active rh-managedredis.service)" == inactive ]]
podman image exists "$SOURCE"
echo P8AR_NATIVE_ROOTLESS_APP_INSTALL_NO_AUTOSTART_PASS

# Product API issues actual native user-systemd operations, not manual copies.
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab managedredis
for i in $(seq 1 90); do
  if podman exec rh-managedredis redis-cli ping 2>/dev/null | grep -Fx PONG; then break; fi
  sleep .5
done
podman exec rh-managedredis redis-cli ping | grep -Fx PONG
test "$(podman inspect rh-managedredis --format '{{.HostConfig.NetworkMode}}')" = none
test "$(podman inspect rh-managedredis --format '{{.HostConfig.ReadonlyRootfs}}')" = true
podman exec rh-managedredis redis-cli SET manager-proof stable | grep -Fx OK
podman exec rh-managedredis redis-cli SAVE | grep -Fx OK
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --stop-lab managedredis
! podman ps --format '{{.Names}}' | grep -Fx rh-managedredis
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab managedredis
for i in $(seq 1 90); do
  if podman exec rh-managedredis redis-cli ping 2>/dev/null | grep -Fx PONG; then break; fi
  sleep .5
done
test "$(podman exec rh-managedredis redis-cli GET manager-proof)" = stable
echo P8AR_RUST_MANAGER_NATIVE_START_STOP_PERSISTENT_APP_DATA_PASS

# Install/remove are deliberately distinct from persistent Podman data.
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab managedredis
! test -e "$QUADLETS/rh-managedredis.container"
! test -e "$QUADLETS/rh-managedredis-data.volume"
! podman ps --format '{{.Names}}' | grep -Fx rh-managedredis
podman volume exists rh-managedredis-data
echo P8AR_RUST_MANAGER_REMOVE_PRESERVES_USER_APP_VOLUME_PASS

# Fail-closed negative controls for unsafe input/host targeting.
if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-lab evil   docker.io/library/redis:latest 999; then exit 1; fi
if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-lab rootuid "$SOURCE" 0; then exit 1; fi
if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab '../bad'; then exit 1; fi
if "$BIN" --start-lab managedredis; then exit 1; fi
test ! -e "$QUADLETS/rh-evil.container"
echo P8AR_APP_MANAGER_REJECTS_MUTABLE_AND_UNAUTHORIZED_INPUT_PASS
