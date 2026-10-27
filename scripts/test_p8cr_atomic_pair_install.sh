#!/usr/bin/env bash
# P8cr: isolated rootless two-App native descriptor transaction.
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
manager() { HARS_DISPOSABLE_OCI_LAB=1 "$BIN" "$@"; }
cleanup() {
  manager --remove-lab p8cr-child >/dev/null 2>&1 || true
  manager --remove-lab p8cr-provider >/dev/null 2>&1 || true
}
trap cleanup EXIT
for slug in p8cr-provider p8cr-child; do
  test ! -e "$QD/rh-$slug.container"
  test ! -e "$QD/rh-$slug-data.volume"
done
podman pull --quiet docker.io/library/redis:7.4-alpine
IMAGE="$(podman image inspect docker.io/library/redis:7.4-alpine --format '{{index .RepoDigests 0}}')"
[[ "$IMAGE" == docker.io/library/redis@sha256:* ]]
MISSING="docker.io/library/redis@sha256:$(printf '0%.0s' {1..64})"
if manager --install-pair-dependent-lab p8cr-provider "$IMAGE" p8cr-child "$MISSING" 999; then
  echo 'missing second image accepted' >&2; exit 1
fi
for slug in p8cr-provider p8cr-child; do
  test ! -e "$QD/rh-$slug.container"
  test ! -e "$QD/rh-$slug-data.volume"
done
echo P8CR_SECOND_IMAGE_PREFLIGHT_REFUSES_BOTH_APP_DESCRIPTORS_PASS

manager --install-pair-dependent-lab p8cr-provider "$IMAGE" p8cr-child "$IMAGE" 999
for slug in p8cr-provider p8cr-child; do
  test "$(stat -c %a "$QD/rh-$slug.container")" = 600
  test "$(stat -c %a "$QD/rh-$slug-data.volume")" = 600
  grep -Fx 'Network=none' "$QD/rh-$slug.container"
  grep -Fx 'ReadOnly=true' "$QD/rh-$slug.container"
  [[ "$(systemctl --user is-active rh-$slug.service || true)" == inactive ]]
done
grep -Fx 'BindsTo=rh-p8cr-provider.service' "$QD/rh-p8cr-child.container"
grep -Fx 'After=rh-p8cr-provider.service' "$QD/rh-p8cr-child.container"
echo P8CR_ATOMIC_TWO_APP_NATIVE_DESCRIPTORS_INERT_AND_PRIVATE_PASS

old="$(sha256sum "$QD/rh-p8cr-provider.container" | cut -d' ' -f1)"
if manager --install-pair-dependent-lab p8cr-provider "$IMAGE" p8cr-child "$IMAGE" 999; then
  echo 'existing pair overwritten' >&2; exit 1
fi
test "$(sha256sum "$QD/rh-p8cr-provider.container" | cut -d' ' -f1)" = "$old"
echo P8CR_REINSTALL_REFUSED_EXISTING_DESCRIPTORS_PRESERVED_PASS

manager --start-lab p8cr-child
for i in $(seq 1 90); do
  if podman exec rh-p8cr-provider redis-cli ping 2>/dev/null | grep -qx PONG &&
     podman exec rh-p8cr-child redis-cli ping 2>/dev/null | grep -qx PONG; then break; fi
  sleep .5
done
for slug in p8cr-provider p8cr-child; do
  test "$(systemctl --user is-active rh-$slug.service)" = active
  test "$(podman inspect "rh-$slug" --format '{{.HostConfig.NetworkMode}}')" = none
  test "$(podman inspect "rh-$slug" --format '{{.HostConfig.ReadonlyRootfs}}')" = true
  test "$(podman inspect "rh-$slug" --format '{{.HostConfig.Privileged}}')" = false
  test "$(podman inspect "rh-$slug" --format '{{.Config.User}}')" = 999
  podman exec "rh-$slug" redis-cli SET "$slug" persistent | grep -Fx OK
  podman exec "rh-$slug" redis-cli SAVE | grep -Fx OK
done
echo P8CR_DEPENDENT_ONLY_START_TWO_REAL_ISOLATED_REDIS_APPS_PASS

if manager --remove-lab p8cr-provider; then
  echo 'provider removed with live dependent' >&2; exit 1
fi
test "$(systemctl --user is-active rh-p8cr-provider.service)" = active
manager --remove-lab p8cr-child
manager --remove-lab p8cr-provider
for slug in p8cr-provider p8cr-child; do
  test ! -e "$QD/rh-$slug.container"
  test ! -e "$QD/rh-$slug-data.volume"
  podman volume exists "rh-$slug-data"
done
echo P8CR_ORDERED_UNINSTALL_RETAINS_BOTH_PRIVATE_DATABASE_VOLUMES_PASS
