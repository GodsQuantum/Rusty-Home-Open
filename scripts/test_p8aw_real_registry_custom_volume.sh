#!/usr/bin/env bash
# Disposable GitHub Ubuntu only. Real third distinct OCI service with native
# generic mounted Podman named volume /var/lib/registry and local-only HTTP.
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
test ! -e "$QD/rh-oci-registry.container"
test ! -e "$QD/rh-oci-registry-restore.container"
cleanup() {
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab oci-registry-restore >/dev/null 2>&1 || true
  HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab oci-registry >/dev/null 2>&1 || true
}
trap cleanup EXIT
podman pull --quiet docker.io/library/registry:3.1.2
IMAGE="$(podman image inspect docker.io/library/registry:3.1.2 --format '{{index .RepoDigests 0}}')"
[[ "$IMAGE" == docker.io/library/registry@sha256:* ]]
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-path-lab    oci-registry "$IMAGE" 1000 /var/lib/registry 18884:5000
test -f "$QD/rh-oci-registry.container"
test "$(stat -c %a "$QD/rh-oci-registry.container")" = 600
grep -F 'Volume=rh-oci-registry-data.volume:/var/lib/registry:U' "$QD/rh-oci-registry.container"
! grep -F 'Volume=rh-oci-registry-data.volume:/data' "$QD/rh-oci-registry.container"
test "$(systemctl --user is-active rh-oci-registry.service)" = inactive
echo P8AW_CUSTOM_NAMED_VOLUME_REGISTRY_INSTALL_NO_AUTOSTART_PASS
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab oci-registry
test "$(podman inspect rh-oci-registry --format '{{.HostConfig.NetworkMode}}')" != none
test "$(podman inspect rh-oci-registry --format '{{.HostConfig.ReadonlyRootfs}}')" = true
test "$(podman inspect rh-oci-registry --format '{{.Config.User}}')" = 1000
test "$(podman port rh-oci-registry 5000/tcp)" = 127.0.0.1:18884
podman inspect rh-oci-registry --format '{{range .Mounts}}{{println .Destination}}{{end}}'  | grep -Fx /var/lib/registry
echo P8AW_REAL_REGISTRY_NAMED_VOLUME_AND_LOCALHOST_ONLY_BIND_PASS

# Docker Distribution v2 blob upload API, using local-only in memory tokens.
# The server stores this genuine artifact in /var/lib/registry (not /data).
python3 scripts/p8aw_registry_http_proof.py --upload

# Restart actual service. The native named-volume retained previous blob.
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --stop-lab oci-registry
! podman ps --format '{{.Names}}' | grep -Fx rh-oci-registry
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab oci-registry
python3 scripts/p8aw_registry_http_proof.py --verify
podman volume exists rh-oci-registry-data
echo P8AW_ACTUAL_DOCKER_REGISTRY_BLOB_SURVIVES_RUST_APP_RESTART_PASS

source scripts/test_p8ax_registry_independent_restore.sh

# Safety: path traversal and Podman system directory overrides are rejected
# before application descriptors are created.
if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-path-lab poison "$IMAGE"     1000 /var/lib/../../host 18885:5000; then exit 1; fi
if HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-path-lab poison "$IMAGE"     1000 /etc 18885:5000; then exit 1; fi
test ! -e "$QD/rh-poison.container"
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab oci-registry
! test -e "$QD/rh-oci-registry.container"
podman volume exists rh-oci-registry-data
echo P8AW_REGISTRY_CUSTOM_VOLUME_UNINSTALL_PRESERVES_BLOBS_PASS
