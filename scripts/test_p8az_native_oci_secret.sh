#!/usr/bin/env bash
set -euo pipefail
[[ "${GITHUB_ACTIONS:-}" == true && -n "${RUNNER_TEMP:-}" && "$(id -u)" != 0 ]] || exit 2
cd "$(dirname "$0")/.."
export XDG_RUNTIME_DIR="/run/user/$(id -u)"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
test -S "$XDG_RUNTIME_DIR/bus"
podman info --format '{{.Host.CgroupsVersion}}' | grep -Fx v2
podman pull --quiet docker.io/library/redis:7.4-alpine
IMAGE="$(podman image inspect docker.io/library/redis:7.4-alpine --format '{{index .RepoDigests 0}}')"
[[ "$IMAGE" == docker.io/library/redis@sha256:* ]]
BIN="$PWD/target/debug/oci_app_manager_lab"
test -x "$BIN"
SECRET="rh-lab-credential"
APP="rh-secretdemo"
UNITS="$HOME/.config/containers/systemd"
test ! -e "$UNITS/$APP.container"
# This is fictional fixture data generated in the disposable runner, NOT an
# actual password or a credential from the developer/production environment.
MARKER="p8az-fictional-secret-fixture-$(date +%s)-$$"
EXPECTED_SHA="$(printf %s "$MARKER" | sha256sum | cut -d' ' -f1)"
cleanup() {
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab secretdemo >/dev/null 2>&1 || true
  podman secret rm "$SECRET" >/dev/null 2>&1 || true
}
trap cleanup EXIT
printf %s "$MARKER" | podman secret create "$SECRET" - >/dev/null
podman secret exists "$SECRET"
# Unknown references and injected shell/Quadlet keys must fail BEFORE install.
if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-secret-lab secretdemo "$IMAGE" 999 unknowncredential; then exit 1; fi
if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-secret-lab secretdemo "$IMAGE" 999 $'evil\n[Service]'; then exit 1; fi
test ! -e "$UNITS/$APP.container"
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-secret-lab secretdemo "$IMAGE" 999 "$SECRET"
test "$(stat -c %a "$UNITS/$APP.container")" = 600
grep -Fx "Secret=$SECRET" "$UNITS/$APP.container"
! grep -Fq "$MARKER" "$UNITS/$APP.container"
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab secretdemo
for i in $(seq 1 90); do
  if podman exec "$APP" redis-cli ping 2>/dev/null | grep -Fx PONG; then break; fi
  sleep .5
done
podman exec "$APP" redis-cli ping | grep -Fx PONG
MOUNTED_SHA="$(podman exec "$APP" sha256sum "/run/secrets/$SECRET" | cut -d' ' -f1)"
test "$MOUNTED_SHA" = "$EXPECTED_SHA"
test "$(podman inspect "$APP" --format '{{.HostConfig.ReadonlyRootfs}}')" = true
test "$(podman inspect "$APP" --format '{{.HostConfig.NetworkMode}}')" = none
printf 'P8AZ_REAL_ROOTLESS_PODMAN_SECRET_MOUNT_SHA256_PASS\n'
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab secretdemo
! test -e "$UNITS/$APP.container"
# Managed service removal must not delete or read an independently owned secret.
podman secret exists "$SECRET"
echo P8AZ_SECRET_LIFECYCLE_PRESERVES_ORIGINAL_SECRET_STORE_PASS
