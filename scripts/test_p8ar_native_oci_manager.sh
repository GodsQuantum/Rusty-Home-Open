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
if [[ "$(printenv HARS_LOCAL_TCP_LAB || true)" == 1 ]]; then
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-lab managedredis "$SOURCE" 999 --local-tcp 18879:6379
else
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-lab managedredis "$SOURCE" 999
fi
test -f "$QUADLETS/rh-managedredis.container"
test -f "$QUADLETS/rh-managedredis-data.volume"
test "$(stat -c '%a' "$QUADLETS/rh-managedredis.container")" = 600
state="$(systemctl --user is-enabled rh-managedredis.service)"
[[ "$state" == static || "$state" == generated ]]
[[ "$state" != enabled ]]
[[ "$(systemctl --user is-active rh-managedredis.service)" == inactive ]]
podman image exists "$SOURCE"
echo P8AR_NATIVE_ROOTLESS_APP_INSTALL_NO_AUTOSTART_PASS

# Hold the true per-App inode flock in this CI shell while the Rust manager
# runs in a distinct process. A start must not race another lifecycle write.
exec 9<>"$QUADLETS/rh-managedredis.lock"
flock --exclusive 9
unit_hash_before="$(sha256sum "$QUADLETS/rh-managedredis.container" | cut -d' ' -f1)"
if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab managedredis; then
  echo 'start unexpectedly bypassed exclusive per-App lock' >&2; exit 1
fi
test "$(sha256sum "$QUADLETS/rh-managedredis.container" | cut -d' ' -f1)" = "$unit_hash_before"
[[ "$(systemctl --user is-active rh-managedredis.service)" == inactive ]]
flock --unlock 9
exec 9>&-
echo P8AY_TRUE_INTERPROCESS_LOCK_DENIES_START_WITHOUT_MODIFICATION_PASS

# Product API issues actual native user-systemd operations, not manual copies.
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab managedredis
for i in $(seq 1 90); do
  if podman exec rh-managedredis redis-cli ping 2>/dev/null | grep -Fx PONG; then break; fi
  sleep .5
done
podman exec rh-managedredis redis-cli ping | grep -Fx PONG
test "$(podman inspect rh-managedredis --format '{{.HostConfig.ReadonlyRootfs}}')" = true
if [[ "$(printenv HARS_LOCAL_TCP_LAB || true)" == 1 ]]; then
  # An explicit network mode must differ from the default fully-offline App.
  test "$(podman inspect rh-managedredis --format '{{.HostConfig.NetworkMode}}')" != none
else
  test "$(podman inspect rh-managedredis --format '{{.HostConfig.NetworkMode}}')" = none
fi
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

if [[ "$(printenv HARS_UPGRADE_ROLLBACK_LAB || true)" == 1 ]]; then
  # Genuine different OCI image and same Redis series, so on-disk RDB remains
  # compatible when the first image is restored.
  podman pull --quiet docker.io/library/redis:7.4-bookworm
  NEXT="$(podman image inspect docker.io/library/redis:7.4-bookworm --format '{{index .RepoDigests 0}}')"
  [[ "$NEXT" == docker.io/library/redis@sha256:* && "$NEXT" != "$SOURCE" ]]
  original_id="$(podman image inspect "$SOURCE" --format '{{.Id}}')"
  updated_id="$(podman image inspect "$NEXT" --format '{{.Id}}')"
  [[ "$original_id" != "$updated_id" ]]
  unit="$QUADLETS/rh-managedredis.container"
  old_unit_hash="$(sha256sum "$unit" | cut -d' ' -f1)"
  # Wrong expected old digest and mutable tags are refused BEFORE any stop.
  if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --switch-image-lab managedredis      "$NEXT" "$SOURCE"; then exit 1; fi
  if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --switch-image-lab managedredis      "$SOURCE" docker.io/library/redis:7.4-bookworm; then exit 1; fi
  test "$(sha256sum "$unit" | cut -d' ' -f1)" = "$old_unit_hash"
  test "$(podman inspect rh-managedredis --format '{{.Image}}')" = "$original_id"
  echo P8AT_MUTABLE_OR_STALE_OCI_UPGRADE_REJECTED_NO_MUTATION_PASS

  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --switch-image-lab managedredis "$SOURCE" "$NEXT"
  for i in $(seq 1 90); do
    if podman exec rh-managedredis redis-cli ping 2>/dev/null | grep -Fx PONG; then break; fi
    sleep .5
  done
  test "$(podman inspect rh-managedredis --format '{{.Image}}')" = "$updated_id"
  test "$(podman exec rh-managedredis redis-cli GET manager-proof)" = stable
  test -f "$QUADLETS/rh-managedredis.rollback"
  podman exec rh-managedredis redis-cli SET manager-proof upgraded-data | grep -Fx OK
  podman exec rh-managedredis redis-cli SAVE | grep -Fx OK
  echo P8AT_REAL_DIGEST_PINNED_IMAGE_SWITCH_PERSISTENT_DATA_PASS

  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --rollback-image-lab managedredis
  for i in $(seq 1 90); do
    if podman exec rh-managedredis redis-cli ping 2>/dev/null | grep -Fx PONG; then break; fi
    sleep .5
  done
  test "$(podman inspect rh-managedredis --format '{{.Image}}')" = "$original_id"
  test "$(podman exec rh-managedredis redis-cli GET manager-proof)" = upgraded-data
  test "$(sha256sum "$unit" | cut -d' ' -f1)" = "$old_unit_hash"
  ! test -e "$QUADLETS/rh-managedredis.rollback"
  if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --rollback-image-lab managedredis; then exit 1; fi
  echo P8AT_REAL_OCI_IMAGE_ROLLBACK_EXACT_UNIT_AND_DATA_RETAINED_PASS
fi


if [[ "$(printenv HARS_LOCAL_TCP_LAB || true)" == 1 ]]; then
  bound="$(podman port rh-managedredis 6379/tcp)"
  printf 'P8AS_NATIVE_PODMAN_HOST_BIND=%s\n' "$bound"
  test "$bound" = '127.0.0.1:18879'
  # Redis itself requires authentication for non-container-loopback clients.
  # Generate an ephemeral random test password, NEVER log it or weaken
  # protected-mode for host access.
  redis_pw="$(python3 -c 'import secrets; print(secrets.token_urlsafe(40))')"
  podman exec rh-managedredis redis-cli CONFIG SET requirepass "$redis_pw" >/dev/null
  P8AS_REDIS_PASSWORD="$redis_pw" python3 - <<'PYTHON'
import os, socket
pw=os.environ["P8AS_REDIS_PASSWORD"].encode("ascii")
auth=b"*2\r\n$4\r\nAUTH\r\n$"+str(len(pw)).encode()+b"\r\n"+pw+b"\r\n"
with socket.create_connection(("127.0.0.1",18879),timeout=5) as conn:
    conn.sendall(auth)
    assert conn.recv(64)==b"+OK\r\n"
    conn.sendall(b"*1\r\n$4\r\nPING\r\n")
    assert conn.recv(64)==b"+PONG\r\n"
PYTHON
  unset redis_pw
  echo P8AS_REAL_ROOTLESS_REDIS_LOCALHOST_ONLY_TCP_PASS
fi

if [[ "$(printenv HARS_HEALTH_ROLLBACK_LAB || true)" == 1 ]]; then
  podman pull --quiet docker.io/library/redis:7.4-bookworm
  GOOD_NEXT="$(podman image inspect docker.io/library/redis:7.4-bookworm --format '{{index .RepoDigests 0}}')"
  podman pull --quiet docker.io/library/busybox:1.36
  BROKEN_NEXT="$(podman image inspect docker.io/library/busybox:1.36 --format '{{index .RepoDigests 0}}')"
  if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-health-lab poison       "$SOURCE" 999 'redis-cli; rm -rf /data'; then exit 1; fi
  test ! -e "$QUADLETS/rh-poison.container"

  # Replace the original offline app definition with native Podman health
  # checks while retaining the pre-existing private named data volume.
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab managedredis
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-health-lab managedredis "$SOURCE" 999 'redis-cli ping'
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab managedredis
  for i in $(seq 1 90); do
    if podman healthcheck run rh-managedredis >/dev/null 2>&1; then break; fi
    sleep .5
  done
  podman healthcheck run rh-managedredis
  test "$(podman exec rh-managedredis redis-cli GET manager-proof)" = stable
  original_id="$(podman image inspect "$SOURCE" --format '{{.Id}}')"
  before="$(sha256sum "$QUADLETS/rh-managedredis.container" | cut -d' ' -f1)"
  echo P8AU_NATIVE_GENERIC_HEALTHCHECK_ON_REAL_REDIS_PASS

  # A cached but incompatible digest (BusyBox) cannot pass the configured
  # Redis-specific health command. Native Rust must recover original image.
  if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --switch-image-lab       managedredis "$SOURCE" "$BROKEN_NEXT"; then
     echo 'Bad image erroneously passed healthcheck' >&2; exit 1
  fi
  for i in $(seq 1 90); do
    if podman exec rh-managedredis redis-cli ping 2>/dev/null | grep -Fx PONG; then break; fi
    sleep .5
  done
  test "$(podman inspect rh-managedredis --format '{{.Image}}')" = "$original_id"
  test "$(podman exec rh-managedredis redis-cli GET manager-proof)" = stable
  test "$(sha256sum "$QUADLETS/rh-managedredis.container" | cut -d' ' -f1)" = "$before"
  echo P8AU_UNHEALTHY_OCI_APP_AUTO_IMAGE_RECOVERY_PASS

  # The failure leaves an owner-private retained rollback record; explicit
  # operator reconciliation restores clean upgrade state.
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --rollback-image-lab managedredis
  ! test -e "$QUADLETS/rh-managedredis.rollback"
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --switch-image-lab       managedredis "$SOURCE" "$GOOD_NEXT"
  podman healthcheck run rh-managedredis
  test "$(podman exec rh-managedredis redis-cli GET manager-proof)" = stable
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --rollback-image-lab managedredis
  podman healthcheck run rh-managedredis
  test "$(podman exec rh-managedredis redis-cli GET manager-proof)" = stable
  echo P8AU_REAL_NATIVE_OCI_HEALTHY_UPGRADE_AND_EXPLICIT_REVERT_PASS
fi

# Install/remove are deliberately distinct from persistent Podman data.
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab managedredis
! test -e "$QUADLETS/rh-managedredis.container"
! test -e "$QUADLETS/rh-managedredis-data.volume"
! podman ps --format '{{.Names}}' | grep -Fx rh-managedredis
podman volume exists rh-managedredis-data
echo P8AR_RUST_MANAGER_REMOVE_PRESERVES_USER_APP_VOLUME_PASS

# The inode lock survives uninstall: overlapping install/remove processes
# cannot accidentally claim different locks for the same slug.
lock_inode_before="$(stat -c '%i' "$QUADLETS/rh-managedredis.lock")"
exec 9<>"$QUADLETS/rh-managedredis.lock"
flock --exclusive 9
if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-lab managedredis "$SOURCE" 999; then
  echo 'install unexpectedly bypassed exclusive per-App lock' >&2; exit 1
fi
test ! -e "$QUADLETS/rh-managedredis.container"
flock --unlock 9
exec 9>&-
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-lab managedredis "$SOURCE" 999
[[ "$lock_inode_before" == "$(stat -c '%i' "$QUADLETS/rh-managedredis.lock")" ]]
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab managedredis
podman volume exists rh-managedredis-data
echo P8AY_TRUE_INTERPROCESS_LOCK_PERSISTS_ACROSS_REMOVE_REINSTALL_PASS

# Fail-closed negative controls for unsafe input/host targeting.
if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-lab evil   docker.io/library/redis:latest 999; then exit 1; fi
if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-lab rootuid "$SOURCE" 0; then exit 1; fi
if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab '../bad'; then exit 1; fi
if "$BIN" --start-lab managedredis; then exit 1; fi
test ! -e "$QUADLETS/rh-evil.container"
echo P8AR_APP_MANAGER_REJECTS_MUTABLE_AND_UNAUTHORIZED_INPUT_PASS
