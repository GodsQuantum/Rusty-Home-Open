#!/usr/bin/env bash
# Fourth distinct stateful OCI App, actual official Mosquitto retained messages.
# Must run ONLY on disposable GitHub rootless Podman runners.
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
source_slug=mqtt-persist
restore_slug=mqtt-restore
source_vol=rh-mqtt-persist-data
restore_vol=rh-mqtt-restore-data
source_data=rh-mqtt-persist-extra
restore_data=rh-mqtt-restore-extra
test ! -e "$QD/rh-$source_slug.container"
test ! -e "$QD/rh-$restore_slug.container"
! podman volume exists "$source_vol"
! podman volume exists "$restore_vol"
! podman volume exists "$source_data"
! podman volume exists "$restore_data"
WORK="$(mktemp -d "$(printenv RUNNER_TEMP)/p8cb-mqtt-XXXXXX")"
chmod 0700 "$WORK"
cleanup() {
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$restore_slug" >/dev/null 2>&1 || true
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$source_slug" >/dev/null 2>&1 || true
  podman volume rm "$restore_data" "$source_data" "$restore_vol" "$source_vol" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT
podman pull --quiet docker.io/library/eclipse-mosquitto:2.0
IMAGE="$(podman image inspect docker.io/library/eclipse-mosquitto:2.0 --format '{{index .RepoDigests 0}}')"
[[ "$IMAGE" == docker.io/library/eclipse-mosquitto@sha256:* ]]
podman image exists "$IMAGE"
podman run --rm --network none --entrypoint /bin/sh "$IMAGE" -ec  'command -v mosquitto_pub; command -v mosquitto_sub'
podman volume create "$source_vol" >/dev/null
podman volume create "$source_data" >/dev/null

# Configure synthetic localhost-only anonymous broker inside Network=none,
# sharing its config and persistence in an owner-private named OCI volume.
podman run --rm --network none --read-only --cap-drop ALL   --security-opt no-new-privileges --user 1883   --volume "$source_vol:/mosquitto:U" --entrypoint /bin/sh "$IMAGE" -ec '
    mkdir -p /mosquitto/config /mosquitto/data /mosquitto/log
    cat > /mosquitto/config/mosquitto.conf <<"EOF"
listener 1883 127.0.0.1
allow_anonymous true
persistence true
persistence_location /mosquitto/data/
autosave_interval 1
log_dest stdout
connection_messages false
EOF
    chmod 0600 /mosquitto/config/mosquitto.conf
  '
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-dual-path-lab   "$source_slug" "$IMAGE" 1883 /mosquitto "$source_data" /mosquitto/data
test "$(systemctl --user is-active rh-$source_slug.service)" = inactive
test "$(stat -c %a "$QD/rh-$source_slug.container")" = 600
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$source_slug"
ready() {
  for _ in $(seq 1 50); do
    if podman exec "$1" mosquitto_pub -h 127.0.0.1 -t lab/p8cb/probe -m probe >/dev/null 2>&1; then return 0; fi
    sleep 0.2
  done
  return 1
}
pub() {
  podman exec "$1" mosquitto_pub -h 127.0.0.1 -p 1883 -q 1 -r -t "$2" -m "$3"
}
sub() {
  podman exec "$1" mosquitto_sub -h 127.0.0.1 -p 1883 -t "$2" -C 1 -W 3
}
assert_offline() {
  test "$(podman inspect "$1" --format '{{.HostConfig.NetworkMode}}')" = none
  test "$(podman inspect "$1" --format '{{.HostConfig.ReadonlyRootfs}}')" = true
  test "$(podman inspect "$1" --format '{{.Config.User}}')" = 1883
}
ready rh-mqtt-persist
assert_offline rh-mqtt-persist
test "$(podman inspect rh-mqtt-persist --format '{{range .Mounts}}{{if eq .Destination "/mosquitto/data"}}{{.Name}}{{end}}{{end}}')" = "$source_data"
echo P8CB_NATIVE_EXPLICIT_DATA_VOLUME_OVERRIDES_OFFICIAL_ANONYMOUS_IMAGE_VOLUME_PASS
pub rh-mqtt-persist lab/p8cb/older before-checkpoint
test "$(sub rh-mqtt-persist lab/p8cb/older)" = before-checkpoint
echo P8CB_BOOTSTRAP_REAL_OFFICIAL_MOSQUITTO_AND_RETAINED_PUBLICATION_PASS
# Strict diagnostic of the official image's nested VOLUME declarations.
# All paths and contents here are synthetic disposable CI data.
podman inspect rh-mqtt-persist --format '{{range .Mounts}}{{.Destination}}={{.Name}};{{end}}'
podman exec rh-mqtt-persist /bin/sh -ec '
  echo P8CB_REAL_APP_CONFIG:
  grep -E "^(persistence|persistence_location|listener|allow_anonymous)" /mosquitto/config/mosquitto.conf
  echo P8CB_REAL_APP_DB_FILES:
  ls -la /mosquitto/data
'
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --stop-lab "$source_slug"
! podman ps --format '{{.Names}}' | grep -Fx rh-mqtt-persist
umask 077
ARCHIVE="$WORK/mosquitto-private.tar"
podman volume export --output "$ARCHIVE" "$source_data"
CONFIG_ARCHIVE="$WORK/mosquitto-config.tar"
podman volume export --output "$CONFIG_ARCHIVE" "$source_vol"
chmod 0600 "$CONFIG_ARCHIVE"
tar -tf "$CONFIG_ARCHIVE" > "$WORK/config-files.txt"
grep -Eq '(^|/)config/mosquitto.conf$' "$WORK/config-files.txt"
CONFIG_SHA="$(sha256sum "$CONFIG_ARCHIVE" | cut -d' ' -f1)"
echo "$CONFIG_SHA  $CONFIG_ARCHIVE" | sha256sum -c
chmod 0600 "$ARCHIVE"
test -s "$ARCHIVE"
test "$(stat -c %a "$ARCHIVE")" = 600
tar -tf "$ARCHIVE" > "$WORK/archive-content.txt"
if ! grep -Eq '(^|/)mosquitto.db$' "$WORK/archive-content.txt"; then
  echo P8CB_ARCHIVE_MISSING_REAL_MOSQUITTO_DB >&2
  head -30 "$WORK/archive-content.txt" >&2
  exit 1
fi
SHA="$(sha256sum "$ARCHIVE" | cut -d' ' -f1)"
[[ "$SHA" =~ ^[0-9a-f]{64}$ ]]
echo "$SHA  $ARCHIVE" | sha256sum -c
cp "$ARCHIVE" "$WORK/bad.tar"
printf CORRUPTED >> "$WORK/bad.tar"
if echo "$SHA  $WORK/bad.tar" | sha256sum -c >/dev/null 2>&1; then exit 1; fi
echo P8CB_OFFICIAL_MOSQUITTO_STOPPED_PRIVATE_RETAINED_DB_ARCHIVE_SHA_PASS
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$source_slug"
ready rh-mqtt-persist
test "$(sub rh-mqtt-persist lab/p8cb/older)" = before-checkpoint
pub rh-mqtt-persist lab/p8cb/newer after-checkpoint
test "$(sub rh-mqtt-persist lab/p8cb/newer)" = after-checkpoint
echo P8CB_PRIMARY_MQTT_BROKER_ADDED_NEW_RETAINED_AFTER_BACKUP_PASS
! podman volume exists "$restore_vol"
podman volume create "$restore_vol" >/dev/null
podman volume create "$restore_data" >/dev/null
echo "$CONFIG_SHA  $CONFIG_ARCHIVE" | sha256sum -c
podman volume import "$restore_vol" "$CONFIG_ARCHIVE"
echo "$SHA  $ARCHIVE" | sha256sum -c
podman volume import "$restore_data" "$ARCHIVE"
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-dual-path-lab   "$restore_slug" "$IMAGE" 1883 /mosquitto "$restore_data" /mosquitto/data
test "$(systemctl --user is-active rh-$restore_slug.service)" = inactive
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$restore_slug"
ready rh-mqtt-restore
assert_offline rh-mqtt-restore
test "$(podman inspect rh-mqtt-restore --format '{{range .Mounts}}{{if eq .Destination "/mosquitto/data"}}{{.Name}}{{end}}{{end}}')" = "$restore_data"
test "$(sub rh-mqtt-restore lab/p8cb/older)" = before-checkpoint
if podman exec rh-mqtt-restore mosquitto_sub -h 127.0.0.1   -t lab/p8cb/newer -C 1 -W 2 > "$WORK/unauthorized-newer.txt"; then
  echo "Restored broker served a post-checkpoint message" >&2; exit 1
fi
test ! -s "$WORK/unauthorized-newer.txt"
test "$(sub rh-mqtt-persist lab/p8cb/newer)" = after-checkpoint
echo P8CB_FOURTH_STATEFUL_APP_INDEPENDENT_RETAINED_RESTORE_POINT_IN_TIME_PASS
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$restore_slug"
podman volume exists "$restore_vol"
podman volume exists "$restore_data"
test "$(sub rh-mqtt-persist lab/p8cb/newer)" = after-checkpoint
echo P8CB_RESTORED_BROKER_REMOVAL_PRESERVES_ORIGINAL_RETAINED_DATA_PASS
