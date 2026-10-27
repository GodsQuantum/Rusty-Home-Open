#!/usr/bin/env bash
# P8cv: kill ONLY the disposable Rust coordinator mid OCI image upgrade.
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
APP=p8cv-redis
JOURNAL="$QD/.rh-image-switch.journal"
UNIT="$QD/rh-$APP.container"
ROLLBACK="$QD/rh-$APP.rollback"
manager() { HARS_DISPOSABLE_OCI_LAB=1 "$BIN" "$@"; }
cleanup() {
  if [[ -f "$JOURNAL" ]]; then manager --recover-image-switch-lab >/dev/null 2>&1 || true; fi
  manager --remove-lab "$APP" >/dev/null 2>&1 || true
}
trap cleanup EXIT
test ! -e "$JOURNAL"
test ! -e "$UNIT"
podman pull --quiet docker.io/library/redis:7.4-alpine
podman pull --quiet docker.io/library/redis:7.4-bookworm
OLD="$(podman image inspect docker.io/library/redis:7.4-alpine --format '{{index .RepoDigests 0}}')"
NEW="$(podman image inspect docker.io/library/redis:7.4-bookworm --format '{{index .RepoDigests 0}}')"
[[ "$OLD" == docker.io/library/redis@sha256:* && "$NEW" == docker.io/library/redis@sha256:* && "$OLD" != "$NEW" ]]
OLD_ID="$(podman image inspect "$OLD" --format '{{.Id}}')"
NEW_ID="$(podman image inspect "$NEW" --format '{{.Id}}')"
test "$OLD_ID" != "$NEW_ID"
manager --install-lab "$APP" "$OLD" 999
manager --start-lab "$APP"
for _ in $(seq 1 80); do
  if podman exec "rh-$APP" redis-cli ping 2>/dev/null | grep -Fx PONG; then break; fi
  sleep .5
done
podman exec "rh-$APP" redis-cli SET p8cv-proof preserved | grep -Fx OK
podman exec "rh-$APP" redis-cli SAVE | grep -Fx OK
ORIGINAL_SHA="$(sha256sum "$UNIT" | cut -d' ' -f1)"
test "$(podman inspect "rh-$APP" --format '{{.HostConfig.NetworkMode}}')" = none
test "$(podman inspect "rh-$APP" --format '{{.HostConfig.ReadonlyRootfs}}')" = true
test "$(podman inspect "rh-$APP" --format '{{.Config.User}}')" = 999
echo P8CV_ORIGINAL_GENUINE_REDIS_PERSISTED_BEFORE_CRASH_PASS

if HARS_P8CV_KILL_AFTER_SWAP=1 manager --switch-image-lab "$APP" "$OLD" "$NEW"; then
  echo 'expected Rust coordinator SIGKILL' >&2; exit 1
fi
test -f "$JOURNAL"
test "$(stat -c '%a' "$JOURNAL")" = 600
test "$(stat -c '%a' "$ROLLBACK")" = 600
grep -Fx "Image=$NEW" "$UNIT"
! systemctl --user is-active --quiet "rh-$APP.service"
podman volume exists "rh-$APP-data"
echo P8CV_REAL_COORDINATOR_SIGKILL_AFTER_DIGEST_SWAP_DURABLE_JOURNAL_PASS

for op in --start-lab --stop-lab --remove-lab --rollback-image-lab; do
  if manager "$op" "$APP"; then echo "mutator escaped journal: $op" >&2; exit 1; fi
done
if manager --switch-image-lab "$APP" "$NEW" "$OLD"; then exit 1; fi
test -f "$JOURNAL"
echo P8CV_INTERRUPTED_UPGRADE_FENCES_ALL_NATIVE_MUTATORS_PASS

mv "$JOURNAL" "$JOURNAL.saved"
ln -s "$RUNNER_TEMP/absent-p8cv-target" "$JOURNAL"
if manager --recover-image-switch-lab; then echo 'symlink journal accepted' >&2; exit 1; fi
test -L "$JOURNAL"
rm "$JOURNAL"
mv "$JOURNAL.saved" "$JOURNAL"
echo P8CV_SYMLINK_JOURNAL_REFUSED_WITHOUT_MUTATION_PASS

manager --recover-image-switch-lab
test ! -e "$JOURNAL"
test ! -e "$ROLLBACK"
test "$(sha256sum "$UNIT" | cut -d' ' -f1)" = "$ORIGINAL_SHA"
for _ in $(seq 1 80); do
  if podman exec "rh-$APP" redis-cli ping 2>/dev/null | grep -Fx PONG; then break; fi
  sleep .5
done
test "$(podman inspect "rh-$APP" --format '{{.Image}}')" = "$OLD_ID"
test "$(podman exec "rh-$APP" redis-cli GET p8cv-proof)" = preserved
echo P8CV_SEPARATE_PROCESS_RECOVERS_EXACT_OLD_DIGEST_AND_REDIS_RDB_PASS

manager --switch-image-lab "$APP" "$OLD" "$NEW"
test ! -e "$JOURNAL"
test "$(podman inspect "rh-$APP" --format '{{.Image}}')" = "$NEW_ID"
manager --rollback-image-lab "$APP"
test ! -e "$JOURNAL"
test "$(podman exec "rh-$APP" redis-cli GET p8cv-proof)" = preserved
echo P8CV_POST_RECOVERY_NORMAL_SWITCH_REGRESSION_PASS
manager --remove-lab "$APP"
podman volume exists "rh-$APP-data"
echo P8CV_CLEANUP_PRESERVES_DISPOSABLE_PRIVATE_DATABASE_PASS
