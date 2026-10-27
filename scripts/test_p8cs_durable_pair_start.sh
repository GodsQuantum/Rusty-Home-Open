#!/usr/bin/env bash
# P8cs: genuine disposable rootless native systemd+Podman crash recovery.
set -euo pipefail
[[ "$GITHUB_ACTIONS" == true && -n "$RUNNER_TEMP" && "$(id -u)" != 0 ]] || exit 2
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export XDG_RUNTIME_DIR="/run/user/$(id -u)"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
test -S "$XDG_RUNTIME_DIR/bus"
BIN="$ROOT/target/debug/oci_app_manager_lab"
test -x "$BIN"
QD="$HOME/.config/containers/systemd"
JOURNAL="$QD/.rh-start-pair.journal"
manager() { HARS_DISPOSABLE_OCI_LAB=1 "$BIN" "$@"; }
cleanup() {
  if [[ -f "$JOURNAL" ]]; then manager --recover-start-pair-lab >/dev/null 2>&1 || true; fi
  if [[ -L "$JOURNAL" ]]; then rm -f "$JOURNAL"; fi
  manager --remove-lab p8cs-child >/dev/null 2>&1 || true
  manager --remove-lab p8cs-provider >/dev/null 2>&1 || true
}
trap cleanup EXIT
test ! -e "$JOURNAL"
for slug in p8cs-provider p8cs-child; do
  test ! -e "$QD/rh-$slug.container"
  test ! -e "$QD/rh-$slug-data.volume"
done
podman pull --quiet docker.io/library/redis:7.4-alpine
IMAGE="$(podman image inspect docker.io/library/redis:7.4-alpine --format '{{index .RepoDigests 0}}')"
[[ "$IMAGE" == docker.io/library/redis@sha256:* ]]
manager --install-pair-dependent-lab p8cs-provider "$IMAGE" p8cs-child "$IMAGE" 999
manager --start-pair-transaction-lab p8cs-provider p8cs-child
test ! -e "$JOURNAL"
for slug in p8cs-provider p8cs-child; do
  test "$(systemctl --user is-active rh-$slug.service)" = active
  test "$(podman inspect rh-$slug --format '{{.HostConfig.NetworkMode}}')" = none
  test "$(podman inspect rh-$slug --format '{{.HostConfig.ReadonlyRootfs}}')" = true
  test "$(podman inspect rh-$slug --format '{{.HostConfig.Privileged}}')" = false
  test "$(podman inspect rh-$slug --format '{{.Config.User}}')" = 999
  podman exec "rh-$slug" redis-cli SET "$slug" durable-value | grep -Fx OK
  podman exec "rh-$slug" redis-cli SAVE | grep -Fx OK
done
if manager --recover-start-pair-lab; then echo 'recovery without journal accepted' >&2; exit 1; fi
echo P8CS_NORMAL_TRANSACTION_COMMITS_TWO_REAL_APPS_AND_PERSISTS_DATA_PASS

manager --stop-lab p8cs-child
manager --stop-lab p8cs-provider
# SIGKILL is delivered to the coordinator process itself, not to Redis.
if HARS_P8CS_KILL_AFTER_PROVIDER=1 manager --start-pair-transaction-lab p8cs-provider p8cs-child; then
  echo 'injected coordinator SIGKILL unexpectedly succeeded' >&2; exit 1
fi
test -f "$JOURNAL"
test "$(stat -c %a "$JOURNAL")" = 600
grep -Fx P8CS1 "$JOURNAL"
grep -Fx p8cs-provider "$JOURNAL"
grep -Fx p8cs-child "$JOURNAL"
test "$(systemctl --user is-active rh-p8cs-provider.service)" = active
! systemctl --user is-active --quiet rh-p8cs-child.service
echo P8CS_GENUINE_COORDINATOR_SIGKILL_LEAVES_DURABLE_INTENT_AND_PARTIAL_RUNTIME_PASS

if manager --start-lab p8cs-child || manager --stop-lab p8cs-provider ||
   manager --remove-lab p8cs-child || manager --start-pair-transaction-lab p8cs-provider p8cs-child; then
  echo 'pending journal bypass accepted' >&2; exit 1
fi
test "$(systemctl --user is-active rh-p8cs-provider.service)" = active
echo P8CS_PENDING_INTENT_FENCES_ALL_NATIVE_APP_MUTATORS_PASS

manager --recover-start-pair-lab
test ! -e "$JOURNAL"
! systemctl --user is-active --quiet rh-p8cs-provider.service
! systemctl --user is-active --quiet rh-p8cs-child.service
for slug in p8cs-provider p8cs-child; do
  podman volume exists "rh-$slug-data"
done
echo P8CS_NEW_PROCESS_RECOVERS_INCOMPLETE_PAIR_WITHOUT_REMOVING_VOLUMES_PASS

manager --start-pair-transaction-lab p8cs-provider p8cs-child
for slug in p8cs-provider p8cs-child; do
  test "$(systemctl --user is-active rh-$slug.service)" = active
  test "$(podman exec rh-$slug redis-cli GET "$slug")" = durable-value
done
test ! -e "$JOURNAL"
echo P8CS_RETRY_RESTORES_BOTH_PERSISTED_REDIS_DATABASES_PASS

# A forged symlink journal must never trigger stops or file traversal.
printf 'untouched\n' > "$RUNNER_TEMP/p8cs-canary"
ln -s "$RUNNER_TEMP/p8cs-canary" "$JOURNAL"
if manager --recover-start-pair-lab; then echo 'symlink journal accepted' >&2; exit 1; fi
test "$(cat "$RUNNER_TEMP/p8cs-canary")" = untouched
test "$(systemctl --user is-active rh-p8cs-provider.service)" = active
test "$(systemctl --user is-active rh-p8cs-child.service)" = active
rm "$JOURNAL"
echo P8CS_FORGED_SYMLINK_JOURNAL_REFUSED_WITHOUT_TOUCHING_LIVE_PAIR_PASS

manager --remove-lab p8cs-child
manager --remove-lab p8cs-provider
for slug in p8cs-provider p8cs-child; do
  podman volume exists "rh-$slug-data"
done
echo P8CS_ORDERED_CLEANUP_PRESERVES_PRIVATE_DATA_VOLUMES_PASS
