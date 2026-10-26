#!/usr/bin/env bash
# Third genuinely distinct stateful OCI App, PostgreSQL 17 Alpine, ephemeral
# GitHub rootless Podman user-systemd only. Do not run on Cloud9/prod/VM100.
set -euo pipefail
[[ "$(printenv GITHUB_ACTIONS || true)" == true ]] || exit 2
[[ -n "$(printenv RUNNER_TEMP || true)" && "$(id -u)" != 0 ]] || exit 2
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export XDG_RUNTIME_DIR="/run/user/$(id -u)"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
test -S "$XDG_RUNTIME_DIR/bus"
test "$(podman info --format '{{.Host.CgroupsVersion}}')" = v2
BIN="$ROOT/target/debug/oci_app_manager_lab"
test -x "$BIN"
QD="$HOME/.config/containers/systemd"
original_slug=pg-sql-proof
restore_slug=pg-sql-restore
orig_vol=rh-pg-sql-proof-data
restore_vol=rh-pg-sql-restore-data
for slug in "$original_slug" "$restore_slug"; do
    test ! -e "$QD/rh-$slug.container"
    test ! -e "$QD/rh-$slug-data.volume"
done
! podman volume exists "$orig_vol"
! podman volume exists "$restore_vol"
P8AY_TMP="$(mktemp -d "$RUNNER_TEMP/p8ay-psql-XXXXXX")"
chmod 0700 "$P8AY_TMP"
cleanup() {
    podman stop pg-sql-bootstrap >/dev/null 2>&1 || true
    HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$restore_slug" >/dev/null 2>&1 || true
    HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$original_slug" >/dev/null 2>&1 || true
    # This is a disposable CI runner: remove only this test's named volumes.
    podman volume rm "$restore_vol" "$orig_vol" >/dev/null 2>&1 || true
    rm -rf "$P8AY_TMP"
}
trap cleanup EXIT
podman pull --quiet docker.io/library/postgres:17-alpine
IMAGE="$(podman image inspect docker.io/library/postgres:17-alpine --format '{{index .RepoDigests 0}}')"
[[ "$IMAGE" == docker.io/library/postgres@sha256:* ]]
podman image exists "$IMAGE"
podman volume create "$orig_vol" >/dev/null

# Official upstream native entrypoint initializes PostgreSQL *once* offline.
# Dummy lab password only during bootstrap, never in persistent Quadlet.
# Rootless UID=70 official Alpine postgres user; loopback-only Unix socket,
# NO public TCP/network, immutable digest, read-only image and tmpfs.
podman run --detach --rm --name pg-sql-bootstrap     --network none --read-only --cap-drop ALL --security-opt no-new-privileges     --pids-limit 128 --memory 256m --user 70     --tmpfs /var/run/postgresql:rw,nosuid,nodev,mode=1777,size=16m     --volume "$orig_vol:/var/lib/postgresql/data:U"     --env POSTGRES_PASSWORD="p8ay-disposable-not-real" "$IMAGE" >/dev/null
ready() {
    local who="$1"
    for _ in $(seq 1 100); do
        if podman exec "$who" pg_isready -q -U postgres -h /var/run/postgresql 2>/dev/null; then return 0; fi
        sleep 0.2
    done
    return 1
}
sql() {
    podman exec "$1" psql --no-psqlrc -X -q -t -A         --set ON_ERROR_STOP=1 -h /var/run/postgresql -U postgres -d postgres -c "$2"
}
ready pg-sql-bootstrap
sql pg-sql-bootstrap "CREATE TABLE rusty_p8ay_checkpoint (id integer PRIMARY KEY, payload text NOT NULL);"
sql pg-sql-bootstrap "INSERT INTO rusty_p8ay_checkpoint VALUES (1,'before-checkpoint');"
test "$(sql pg-sql-bootstrap 'SELECT payload FROM rusty_p8ay_checkpoint WHERE id=1;')" = before-checkpoint
podman stop pg-sql-bootstrap >/dev/null
! podman ps --format '{{.Names}}' | grep -Fx pg-sql-bootstrap
echo P8AY_OFFICIAL_PG17_REAL_TRANSACTIONAL_SQL_QUESCED_CHECKPOINT_PASS

# Point-in-time physical PostgreSQL basebackup from stopped writer only.
# Podman volume export/import never accepted from a live DB writer.
umask 077
ARCHIVE="$P8AY_TMP/postgres17-private.tar"
podman volume export --output "$ARCHIVE" "$orig_vol"
chmod 0600 "$ARCHIVE"
test "$(stat -c %a "$ARCHIVE")" = 600
test -s "$ARCHIVE"
tar -tf "$ARCHIVE" | grep -Eq '(^|/)PG_VERSION$'
CHECKSUM="$(sha256sum "$ARCHIVE" | cut -d' ' -f1)"
[[ "$CHECKSUM" =~ ^[0-9a-f]{64}$ ]]
echo "$CHECKSUM  $ARCHIVE" | sha256sum -c
cp "$ARCHIVE" "$P8AY_TMP/tampered.tar"
printf 'CORRUPTED' >> "$P8AY_TMP/tampered.tar"
if echo "$CHECKSUM  $P8AY_TMP/tampered.tar" | sha256sum -c >/dev/null 2>&1; then exit 1; fi
echo P8AY_PG17_PRIVATE_PHYSICAL_VOLUME_SHA_AND_TAMPER_REFUSAL_PASS

# Original is now started by the GENERIC Rust App manager, same default
# upstream postgres entrypoint and digest, not a bespoke daemon wrapper.
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-path-tmpfs-lab     "$original_slug" "$IMAGE" 70 /var/lib/postgresql/data /var/run/postgresql
test "$(systemctl --user is-active rh-$original_slug.service)" = inactive
grep -F 'Tmpfs=/var/run/postgresql:rw,nosuid,nodev,mode=1777,size=16m' "$QD/rh-$original_slug.container"
test "$(stat -c %a "$QD/rh-$original_slug.container")" = 600
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$original_slug"
ready rh-pg-sql-proof
test "$(podman inspect rh-pg-sql-proof --format '{{.HostConfig.NetworkMode}}')" = none
test "$(podman inspect rh-pg-sql-proof --format '{{.HostConfig.ReadonlyRootfs}}')" = true
test "$(podman inspect rh-pg-sql-proof --format '{{.Config.User}}')" = 70
test "$(sql rh-pg-sql-proof 'SELECT payload FROM rusty_p8ay_checkpoint WHERE id=1;')" = before-checkpoint
sql rh-pg-sql-proof "INSERT INTO rusty_p8ay_checkpoint VALUES (2,'after-checkpoint');"
test "$(sql rh-pg-sql-proof 'SELECT count(*) FROM rusty_p8ay_checkpoint;')" = 2
echo P8AY_GENERIC_NATIVE_RUST_MANAGER_WRITES_REAL_POST_CHECKPOINT_SQL_PASS

# A second independent volume must load original snapshot ONLY, without
# replaying the post-checkpoint COMMITTED insert from the still-running source.
! podman volume exists "$restore_vol"
podman volume create "$restore_vol" >/dev/null
echo "$CHECKSUM  $ARCHIVE" | sha256sum -c
podman volume import "$restore_vol" "$ARCHIVE"
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-path-tmpfs-lab     "$restore_slug" "$IMAGE" 70 /var/lib/postgresql/data /var/run/postgresql
test "$(systemctl --user is-active rh-$restore_slug.service)" = inactive
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$restore_slug"
ready rh-pg-sql-restore
test "$(podman inspect rh-pg-sql-restore --format '{{.HostConfig.NetworkMode}}')" = none
test "$(podman inspect rh-pg-sql-restore --format '{{.HostConfig.ReadonlyRootfs}}')" = true
test "$(podman inspect rh-pg-sql-restore --format '{{.Config.User}}')" = 70
test "$(sql rh-pg-sql-restore 'SELECT count(*) FROM rusty_p8ay_checkpoint;')" = 1
test "$(sql rh-pg-sql-restore 'SELECT payload FROM rusty_p8ay_checkpoint WHERE id=1;')" = before-checkpoint
test "$(sql rh-pg-sql-proof 'SELECT count(*) FROM rusty_p8ay_checkpoint;')" = 2
echo P8AY_THIRD_DISTINCT_REAL_PG17_APP_SQL_ROLLBACK_POINT_IN_TIME_PASS

HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$restore_slug"
podman volume exists "$restore_vol"
test "$(sql rh-pg-sql-proof 'SELECT count(*) FROM rusty_p8ay_checkpoint;')" = 2
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$original_slug"
podman volume exists "$orig_vol"
echo P8AY_PG17_INDEPENDENT_DATABASE_APP_REMOVE_PRESERVES_OWN_DATA_PASS
