#!/usr/bin/env bash
# Two DIFFERENT upstream OCI applications run simultaneously under Rust
# generic manager. No app-specific implementation or production host writes.
set -euo pipefail
[[ "$(printenv GITHUB_ACTIONS || true)" == true && -n "$(printenv RUNNER_TEMP || true)" && "$(id -u)" != 0 ]] || exit 2
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export XDG_RUNTIME_DIR="/run/user/$(id -u)"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
test -S "$XDG_RUNTIME_DIR/bus"
test "$(podman info --format '{{.Host.CgroupsVersion}}')" = v2
BIN="$ROOT/target/debug/oci_app_manager_lab"
test -x "$BIN"
QD="$HOME/.config/containers/systemd"
for app in redis-two memorycache; do test ! -e "$QD/rh-$app.container"; done

cleanup(){
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab memorycache >/dev/null 2>&1 || true
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab redis-two >/dev/null 2>&1 || true
}
trap cleanup EXIT

# Two independently maintained official images with distinct OCI App UIDs:
# Redis keeps disk state; Memcached holds deliberately transient cache state.
podman pull --quiet docker.io/library/redis:7.4-alpine
podman pull --quiet docker.io/library/memcached:1.6-alpine
REDIS="$(podman image inspect docker.io/library/redis:7.4-alpine --format '{{index .RepoDigests 0}}')"
CACHE="$(podman image inspect docker.io/library/memcached:1.6-alpine --format '{{index .RepoDigests 0}}')"
[[ "$REDIS" == docker.io/library/redis@sha256:* ]]
[[ "$CACHE" == docker.io/library/memcached@sha256:* ]]
[[ "$REDIS" != "$CACHE" ]]

HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-lab redis-two "$REDIS" 999
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-lab memorycache "$CACHE" 11211
test -f "$QD/rh-redis-two.container"
test -f "$QD/rh-memorycache.container"
test "$(stat -c %a "$QD/rh-redis-two.container")" = 600
test "$(stat -c %a "$QD/rh-memorycache.container")" = 600
test "$(systemctl --user is-active rh-memorycache.service)" = inactive
test "$(systemctl --user is-active rh-redis-two.service)" = inactive
echo P8AV_TWO_DISTINCT_PINNED_OCI_APP_INSTALLS_NO_AUTOSTART_PASS

HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab redis-two
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab memorycache
for i in $(seq 1 60); do
  if podman exec rh-redis-two redis-cli ping 2>/dev/null | grep -qx PONG; then break; fi
  sleep .5
done
podman exec rh-redis-two redis-cli ping | grep -Fx PONG
# Memcached Alpine uses busybox nc to exercise its actual in-namespace TCP.
cache_command(){
  podman exec rh-memorycache sh -c "printf '$1' | nc -w 2 127.0.0.1 11211"
}
for i in $(seq 1 60); do
  if cache_command 'version\r\n' 2>/dev/null | grep -q '^VERSION '; then break; fi
  sleep .5
done
cache_command 'version\r\n' | grep -E '^VERSION 1\.6'
cache_command 'set rustycache 0 0 3\r\none\r\n' | tr -d '\r' | grep -Fx STORED
cache_command 'get rustycache\r\n' | tr -d '\r' | grep -Fx one
podman exec rh-redis-two redis-cli SET rusty-multi independent | grep -Fx OK
podman exec rh-redis-two redis-cli SAVE | grep -Fx OK

for app in redis-two memorycache; do
  test "$(podman inspect "rh-$app" --format '{{.HostConfig.NetworkMode}}')" = none
  test "$(podman inspect "rh-$app" --format '{{.HostConfig.ReadonlyRootfs}}')" = true
  test "$(podman inspect "rh-$app" --format '{{.HostConfig.Privileged}}')" = false
done
test "$(podman inspect rh-redis-two --format '{{.Config.User}}')" = 999
test "$(podman inspect rh-memorycache --format '{{.Config.User}}')" = 11211
echo P8AV_TWO_LIVE_UNRELATED_ROOTLESS_OCI_APPS_ISOLATED_PASS

# Stopping one must not disturb the other.
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --stop-lab memorycache
test "$(podman exec rh-redis-two redis-cli GET rusty-multi)" = independent
test "$(systemctl --user is-active rh-redis-two.service)" = active
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab memorycache
for i in $(seq 1 60); do
  if cache_command 'version\r\n' 2>/dev/null | grep -q '^VERSION '; then break; fi
  sleep .5
done
cache_command 'get rustycache\r\n' | tr -d '\r' | grep -Fx END
test "$(podman exec rh-redis-two redis-cli GET rusty-multi)" = independent
echo P8AV_INDEPENDENT_APP_LIFECYCLE_AND_DATA_DURABILITY_PASS

# Removal is per App and intentionally preserves independent data volumes.
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab memorycache
! test -e "$QD/rh-memorycache.container"
podman volume exists rh-memorycache-data
test "$(podman exec rh-redis-two redis-cli GET rusty-multi)" = independent
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab redis-two
podman volume exists rh-redis-two-data
! test -e "$QD/rh-redis-two.container"
echo P8AV_TWO_GENERIC_APPS_UNINSTALL_WITHOUT_DATA_LOSS_PASS
