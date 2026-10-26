#!/usr/bin/env bash
# Sourced from the original disposable P8aw GitHub-runner harness, NEVER
# from the Cloud9 host. BIN, IMAGE, QD and first OCI App are known.
set -euo pipefail
[[ "$(printenv GITHUB_ACTIONS || true)" == true && -n "$(printenv RUNNER_TEMP || true)" && "$(id -u)" != 0 ]] || exit 2
test "$(podman inspect rh-oci-registry --format '{{.State.Running}}')" = true
original=rh-oci-registry-data
recovered=rh-oci-registry-restore-data
! podman volume exists "$recovered"
test ! -e "$QD/rh-oci-registry-restore.container"

# Export requires a STOPPED writer; Podman volume export is not a DB snapshot.
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --stop-lab oci-registry
! podman ps --format '{{.Names}}' | grep -Fx rh-oci-registry
umask 077
P8AX_TMP="$(mktemp -d "$RUNNER_TEMP/p8ax-private-XXXXXX")"
chmod 0700 "$P8AX_TMP"
snapshot="$P8AX_TMP/distribution.tar"
podman volume export --output "$snapshot" "$original"
chmod 0600 "$snapshot"
test -s "$snapshot"
test "$(stat -c %a "$snapshot")" = 600
tar -tf "$snapshot" | grep -Eq '(^|/)blobs/sha256/'
sha="$(sha256sum "$snapshot" | cut -d' ' -f1)"
[[ "$sha" =~ ^[0-9a-f]{64}$ ]]
echo "$sha  $snapshot" | sha256sum -c
echo P8AX_SECOND_GENUINE_STATEFUL_APP_STOPPED_PRIVATE_VOLUME_EXPORT_PASS

cp "$snapshot" "$P8AX_TMP/tampered.tar"
printf 'CORRUPTED' >> "$P8AX_TMP/tampered.tar"
if echo "$sha  $P8AX_TMP/tampered.tar" | sha256sum -c >/dev/null 2>&1; then
  echo 'Tampered OCI App archive was accepted' >&2; exit 1
fi
echo P8AX_SECOND_APP_ARCHIVE_SHA_TAMPER_REJECTED_PASS

# Fork the primary AFTER the checkpoint with a newly committed OCI blob.
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab oci-registry
HARS_REGISTRY_LAB_PORT=18884 python3 scripts/p8aw_registry_http_proof.py --verify
HARS_REGISTRY_LAB_PORT=18884 python3 scripts/p8aw_registry_http_proof.py --upload-later
echo P8AX_PRIMARY_REGISTRY_MUTATED_AFTER_EXPORT_PASS

# Podman import MERGES: demand a distinct empty volume, never original.
! podman volume exists "$recovered"
podman volume create "$recovered" >/dev/null
echo "$sha  $snapshot" | sha256sum -c
podman volume import "$recovered" "$snapshot"
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-path-lab     oci-registry-restore "$IMAGE" 1000 /var/lib/registry 18885:5000
test "$(stat -c %a "$QD/rh-oci-registry-restore.container")" = 600
test "$(systemctl --user is-active rh-oci-registry-restore.service)" = inactive
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab oci-registry-restore
test "$(podman port rh-oci-registry-restore 5000/tcp)" = 127.0.0.1:18885
test "$(podman inspect rh-oci-registry-restore --format '{{.HostConfig.ReadonlyRootfs}}')" = true
test "$(podman inspect rh-oci-registry-restore --format '{{.Config.User}}')" = 1000

# Distinct actual HTTP servers, same pinned image and different volumes.
HARS_REGISTRY_LAB_PORT=18885 python3 scripts/p8aw_registry_http_proof.py --verify
HARS_REGISTRY_LAB_PORT=18885 python3 scripts/p8aw_registry_http_proof.py --verify-later-absent
HARS_REGISTRY_LAB_PORT=18884 python3 scripts/p8aw_registry_http_proof.py --upload-later
echo P8AX_REGISTRY_INDEPENDENT_PERSISTENT_VOLUME_RESTORE_POINT_IN_TIME_PASS

HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab oci-registry-restore
! test -e "$QD/rh-oci-registry-restore.container"
podman volume exists "$recovered"
test "$(podman inspect rh-oci-registry --format '{{.State.Running}}')" = true
echo P8AX_RESTORED_APP_REMOVAL_PRESERVES_ORIGINAL_AND_BACKUP_DATA_PASS
