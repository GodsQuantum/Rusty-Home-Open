#!/usr/bin/env bash
# P8co: genuine rootless systemd/Podman service dependency identity, not
# an HAOS add-on migration, app readiness protocol or production cutover.
set -euo pipefail
[[ "${GITHUB_ACTIONS:-}" == true && -n "${RUNNER_TEMP:-}" && "$(id -u)" != 0 ]] || exit 2
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export XDG_RUNTIME_DIR="/run/user/$(id -u)"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
test -S "$XDG_RUNTIME_DIR/bus"
test "$(podman info --format '{{.Host.CgroupsVersion}}')" = v2
BIN="$ROOT/target/debug/oci_app_manager_lab"
test -x "$BIN"
QD="$HOME/.config/containers/systemd"
for slug in p8co-provider p8co-consumer p8co-missing; do
  test ! -e "$QD/rh-$slug.container"
done
manager() { HARS_DISPOSABLE_OCI_LAB=1 "$BIN" "$@"; }
cleanup() {
  manager --remove-lab p8co-consumer >/dev/null 2>&1 || true
  manager --remove-lab p8co-provider >/dev/null 2>&1 || true
}
trap cleanup EXIT

podman pull --quiet docker.io/library/redis:7.4-alpine
IMAGE="$(podman image inspect docker.io/library/redis:7.4-alpine --format '{{index .RepoDigests 0}}')"
[[ "$IMAGE" == docker.io/library/redis@sha256:* ]]
# A nonexistent provider must not create a dangling service or volume unit.
if manager --install-dependent-lab p8co-missing "$IMAGE" 999 p8co-absent; then
  echo 'dangling dependency accepted' >&2; exit 1
fi
test ! -e "$QD/rh-p8co-missing.container"
test ! -e "$QD/rh-p8co-missing-data.volume"
echo P8CO_MISSING_PROVIDER_REFUSED_WITHOUT_UNIT_CREATION_PASS

manager --install-lab p8co-provider "$IMAGE" 999
manager --install-dependent-lab p8co-consumer "$IMAGE" 999 p8co-provider
test "$(stat -c %a "$QD/rh-p8co-consumer.container")" = 600
grep -Fx 'BindsTo=rh-p8co-provider.service' "$QD/rh-p8co-consumer.container"
grep -Fx 'After=rh-p8co-provider.service' "$QD/rh-p8co-consumer.container"
grep -Fx 'Network=none' "$QD/rh-p8co-consumer.container"
grep -Fx 'ReadOnly=true' "$QD/rh-p8co-consumer.container"
[[ "$(systemctl --user is-active rh-p8co-provider.service)" == inactive ]]
[[ "$(systemctl --user is-active rh-p8co-consumer.service)" == inactive ]]
echo P8CO_TWO_REAL_APPS_DECLARED_DEPENDENCY_NO_AUTOSTART_PASS

# Starting ONLY the dependent must order and activate the provider.
manager --start-lab p8co-consumer
for i in $(seq 1 90); do
  if podman exec rh-p8co-provider redis-cli ping 2>/dev/null | grep -qx PONG &&
     podman exec rh-p8co-consumer redis-cli ping 2>/dev/null | grep -qx PONG; then break; fi
  sleep .5
done
test "$(systemctl --user is-active rh-p8co-provider.service)" = active
test "$(systemctl --user is-active rh-p8co-consumer.service)" = active
for slug in p8co-provider p8co-consumer; do
  test "$(podman inspect "rh-$slug" --format '{{.HostConfig.NetworkMode}}')" = none
  test "$(podman inspect "rh-$slug" --format '{{.HostConfig.ReadonlyRootfs}}')" = true
  test "$(podman inspect "rh-$slug" --format '{{.HostConfig.Privileged}}')" = false
  test "$(podman inspect "rh-$slug" --format '{{.Config.User}}')" = 999
done
podman exec rh-p8co-provider redis-cli SET p8co-provider-key durable | grep -Fx OK
podman exec rh-p8co-consumer redis-cli SET p8co-consumer-key durable | grep -Fx OK
podman exec rh-p8co-provider redis-cli SAVE | grep -Fx OK
podman exec rh-p8co-consumer redis-cli SAVE | grep -Fx OK
echo P8CO_DEPENDENT_START_ACTIVATES_REAL_PROVIDER_AND_TWO_ISOLATED_DATABASES_PASS

# Provider uninstall must fail while the dependent exists, without touching
# either running service or its exact descriptor bytes.
before="$(sha256sum "$QD/rh-p8co-provider.container" | cut -d' ' -f1)"
if manager --remove-lab p8co-provider; then
  echo 'provider removed while still required' >&2; exit 1
fi
test "$(sha256sum "$QD/rh-p8co-provider.container" | cut -d' ' -f1)" = "$before"
test "$(systemctl --user is-active rh-p8co-provider.service)" = active
test "$(systemctl --user is-active rh-p8co-consumer.service)" = active
echo P8CO_PROVIDER_REMOVAL_REFUSED_WITH_DEPENDENT_LIVE_PASS

# Native BindsTo must cascade a deliberate provider stop to its consumer.
manager --stop-lab p8co-provider
for i in $(seq 1 60); do
  p="$(systemctl --user is-active rh-p8co-provider.service || true)"
  c="$(systemctl --user is-active rh-p8co-consumer.service || true)"
  if [[ "$p" != active && "$p" != activating && "$c" != active && "$c" != activating ]]; then break; fi
  sleep .5
done
[[ "$(systemctl --user is-active rh-p8co-provider.service || true)" == inactive ]]
[[ "$(systemctl --user is-active rh-p8co-consumer.service || true)" == inactive ]]
! podman ps --format '{{.Names}}' | grep -Eq '^rh-p8co-(provider|consumer)$'
echo P8CO_NATIVE_PROVIDER_STOP_CASCADES_DEPENDENT_STOP_PASS

manager --start-lab p8co-consumer
for i in $(seq 1 90); do
  if podman exec rh-p8co-provider redis-cli ping 2>/dev/null | grep -qx PONG &&
     podman exec rh-p8co-consumer redis-cli ping 2>/dev/null | grep -qx PONG; then break; fi
  sleep .5
done
test "$(podman exec rh-p8co-provider redis-cli GET p8co-provider-key)" = durable
test "$(podman exec rh-p8co-consumer redis-cli GET p8co-consumer-key)" = durable
echo P8CO_DEPENDENCY_RESTART_PRESERVES_BOTH_REAL_DATABASES_PASS

manager --remove-lab p8co-consumer
manager --remove-lab p8co-provider
test ! -e "$QD/rh-p8co-consumer.container"
test ! -e "$QD/rh-p8co-provider.container"
podman volume exists rh-p8co-provider-data
podman volume exists rh-p8co-consumer-data
echo P8CO_DEPENDENT_FIRST_CLEANUP_PRESERVES_SEPARATE_VOLUMES_PASS
