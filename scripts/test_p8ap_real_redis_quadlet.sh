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
  systemctl --user stop rh-redis-restore.service >/dev/null 2>&1 || true
  systemctl --user stop rh-redis.service >/dev/null 2>&1 || true
  rm -f "$QUADLETS/rh-redis-restore.container" "$QUADLETS/rh-redis-restore-data.volume"
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

# P8aq: independent, quiesced export and restore of a real OCI App's data.
# Only disposable GitHub runner and opt-in environment, never live household.
if [[ "$(printenv HARS_VOLUME_RESTORE_LAB || true)" == 1 ]]; then
  original_volume=rh-redis-data
  restored_volume=rh-redis-restore-data
  [[ ! -e "$QUADLETS/rh-redis-restore.container" ]] || exit 2
  ! podman volume exists "$restored_volume"
  # Quiesce live Redis writer before the native Podman volume export.
  podman exec rh-redis redis-cli SAVE | grep -Fx OK
  systemctl --user stop rh-redis.service
  ! podman ps --format '{{.Names}}' | grep -Fx rh-redis
  podman volume exists "$original_volume"
  umask 077
  backup="$APP_DIR/rh-redis-data.tar"
  podman volume export --output "$backup" "$original_volume"
  # Podman 4.9 creates output using its own mode: enforce private file
  # permissions explicitly before reading or retaining the archive.
  chmod 0600 "$backup"
  test -s "$backup"
  [[ "$(stat -c '%a' "$backup")" == 600 ]]
  echo P8AQ_PODMAN_VOLUME_EXPORT_PRIVATE_ARCHIVE_PASS
  tar -tf "$backup" | grep -E '(^|/)dump\.rdb$'
  echo P8AQ_REDIS_RDB_PRESENT_IN_ARCHIVE_PASS
  backup_sha="$(sha256sum "$backup" | cut -d' ' -f1)"
  [[ "$backup_sha" =~ ^[0-9a-f]{64}$ ]]
  echo "$backup_sha  $backup" | sha256sum -c
  echo P8AQ_REAL_OFFLINE_APP_BACKUP_VERIFIED_PASS

  # Tampered backup must not be fed into volume import.
  cp "$backup" "$APP_DIR/tampered.tar"
  printf 'CORRUPTED' >> "$APP_DIR/tampered.tar"
  if echo "$backup_sha  $APP_DIR/tampered.tar" | sha256sum -c; then
    echo 'Corrupt backup unexpectedly accepted' >&2
    exit 1
  fi
  echo P8AQ_CORRUPTED_OCI_BACKUP_REJECTED_PASS

  # Mutate the PRIMARY after backup. Restored state must remain earlier.
  systemctl --user start rh-redis.service
  for i in $(seq 1 90); do
    if podman exec rh-redis redis-cli ping 2>/dev/null | grep -Fx PONG; then break; fi
    sleep .5
  done
  podman exec rh-redis redis-cli SET rusty-lab-proof 'post-backup-change' | grep -Fx OK
  podman exec rh-redis redis-cli SAVE | grep -Fx OK
  test "$(podman exec rh-redis redis-cli GET rusty-lab-proof)" = post-backup-change

  # Podman volume import MERGES data; require a NEW destination, never source.
  ! podman volume exists "$restored_volume"
  podman volume create "$restored_volume" >/dev/null
  echo "$backup_sha  $backup" | sha256sum -c
  podman volume import "$restored_volume" "$backup"
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --generate-lab redis-restore "$SOURCE" "$APP_DIR" --uid 999
  install -m 0600 "$APP_DIR/rh-redis-restore.container" "$QUADLETS/rh-redis-restore.container"
  install -m 0600 "$APP_DIR/rh-redis-restore-data.volume" "$QUADLETS/rh-redis-restore-data.volume"
  systemctl --user daemon-reload
  systemctl --user start rh-redis-restore.service
  for i in $(seq 1 90); do
    if podman exec rh-redis-restore redis-cli ping 2>/dev/null | grep -Fx PONG; then break; fi
    sleep .5
  done
  test "$(podman exec rh-redis-restore redis-cli GET rusty-lab-proof)" = persistent-data
  test "$(podman exec rh-redis redis-cli GET rusty-lab-proof)" = post-backup-change
  test "$(podman inspect rh-redis-restore --format '{{.HostConfig.NetworkMode}}')" = none
  test "$(podman inspect rh-redis-restore --format '{{.HostConfig.ReadonlyRootfs}}')" = true
  echo P8AQ_INDEPENDENT_RESTORED_REDIS_DATA_AND_PRIMARY_UNCHANGED_PASS
  systemctl --user stop rh-redis-restore.service
  ! podman ps --format '{{.Names}}' | grep -Fx rh-redis-restore
fi
