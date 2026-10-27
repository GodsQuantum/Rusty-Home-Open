#!/usr/bin/env bash
# Genuine sixth independent stateful OCI App: Grafana dashboards via its
# real authenticated HTTP API, SQLite volume, not just file copy.
# Disposable rootless GitHub Actions runner only.
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
primary=dashboard-source
restored=dashboard-restore
primary_vol=rh-dashboard-source-data
restored_vol=rh-dashboard-restore-data
test ! -e "$QD/rh-$primary.container"
test ! -e "$QD/rh-$restored.container"
! podman volume exists "$primary_vol"
! podman volume exists "$restored_vol"
WORK="$(mktemp -d "$(printenv RUNNER_TEMP)/p8cd-grafana-XXXXXX")"
chmod 0700 "$WORK"
cleanup() {
 HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$restored" >/dev/null 2>&1 || true
 HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$primary" >/dev/null 2>&1 || true
 podman volume rm "$restored_vol" "$primary_vol" >/dev/null 2>&1 || true
 rm -rf "$WORK"
}
trap cleanup EXIT
podman pull --quiet docker.io/grafana/grafana:12.2.0
IMAGE="$(podman image inspect docker.io/grafana/grafana:12.2.0 --format '{{index .RepoDigests 0}}')"
[[ "$IMAGE" == docker.io/grafana/grafana@sha256:* ]]
podman image exists "$IMAGE"

# Default admin/admin is a disposable OFFLINE upstream fixture only; no real
# account, API key, persistently injected credential or exposed TCP service.
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-path-lab  "$primary" "$IMAGE" 472 /var/lib/grafana
test "$(systemctl --user is-active rh-$primary.service)" = inactive
test "$(stat -c %a "$QD/rh-$primary.container")" = 600
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$primary"
offline() {
 test "$(podman inspect "$1" --format '{{.HostConfig.NetworkMode}}')" = none
 test "$(podman inspect "$1" --format '{{.HostConfig.ReadonlyRootfs}}')" = true
 test "$(podman inspect "$1" --format '{{.Config.User}}')" = 472
 test "$(podman inspect "$1" --format '{{range .Mounts}}{{if eq .Destination "/var/lib/grafana"}}{{.Name}}{{end}}{{end}}')" = "$2"
}
ready() {
 for _ in $(seq 1 110); do
  if podman exec "$1" curl --silent --fail --max-time 2    http://127.0.0.1:3000/api/health >/dev/null 2>&1; then return 0; fi
  sleep 0.2
 done
 podman logs "$1" 2>&1 | tail -25 >&2 || true
 return 1
}
dashboard_json() {
 printf '{"dashboard":{"uid":"%s","title":"%s","tags":["p8cd-offline-only"],"timezone":"browser","schemaVersion":40,"panels":[]},"overwrite":false}' "$1" "$2"
}
create_dashboard() {
 local uid="$2" title="$3"
 dashboard_json "$uid" "$title" | podman exec -i "$1" curl --silent --show-error --fail   --max-time 7 --user admin:admin -H 'Content-Type: application/json'   -X POST --data-binary @- http://127.0.0.1:3000/api/dashboards/db   | python3 -c 'import json,sys; j=json.load(sys.stdin);assert j["status"]=="success" and j["uid"]==sys.argv[1]' "$uid"
}
verify_dashboard() {
 local actual
 actual="$(podman exec "$1" curl --silent --show-error --fail --max-time 7   --user admin:admin "http://127.0.0.1:3000/api/dashboards/uid/$2" |
  python3 -c 'import json,sys;j=json.load(sys.stdin);print(j["dashboard"]["title"])')"
 test "$actual" = "$3"
}
absent_dashboard() {
 local code
 code="$(podman exec "$1" curl --silent --max-time 7   --user admin:admin --output /dev/null --write-out '%{http_code}'   "http://127.0.0.1:3000/api/dashboards/uid/$2")"
 test "$code" = 404
}
ready rh-dashboard-source
offline rh-dashboard-source "$primary_vol"
create_dashboard rh-dashboard-source p8cd-before 'Before Snapshot'
verify_dashboard rh-dashboard-source p8cd-before 'Before Snapshot'
echo P8CD_GENUINE_GRAFANA_REAL_DASHBOARD_HTTP_SQLITE_WRITE_PASS
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --stop-lab "$primary"
! podman ps --format '{{.Names}}' | grep -Fx rh-dashboard-source
umask 077
ARCHIVE="$WORK/grafana-sqlite.tar"
podman volume export --output "$ARCHIVE" "$primary_vol"
chmod 0600 "$ARCHIVE"
test -s "$ARCHIVE"
test "$(stat -c %a "$ARCHIVE")" = 600
tar -tf "$ARCHIVE" > "$WORK/archive-files.txt"
grep -Eq '(^|/)grafana.db$' "$WORK/archive-files.txt"
CHECKSUM="$(sha256sum "$ARCHIVE" | cut -d' ' -f1)"
[[ "$CHECKSUM" =~ ^[0-9a-f]{64}$ ]]
echo "$CHECKSUM  $ARCHIVE" | sha256sum -c
cp "$ARCHIVE" "$WORK/corrupted.tar"
printf CORRUPTED >> "$WORK/corrupted.tar"
if echo "$CHECKSUM  $WORK/corrupted.tar" | sha256sum -c >/dev/null 2>&1; then exit 1; fi
echo P8CD_GRAFANA_STOPPED_SQLITE_SHA256_TAMPER_DENIAL_PASS
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$primary"
ready rh-dashboard-source
verify_dashboard rh-dashboard-source p8cd-before 'Before Snapshot'
create_dashboard rh-dashboard-source p8cd-after 'After Snapshot'
verify_dashboard rh-dashboard-source p8cd-after 'After Snapshot'
echo P8CD_SOURCE_GRAFANA_POST_CHECKPOINT_SECOND_DASHBOARD_PASS
! podman volume exists "$restored_vol"
podman volume create "$restored_vol" >/dev/null
echo "$CHECKSUM  $ARCHIVE" | sha256sum -c
podman volume import "$restored_vol" "$ARCHIVE"
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-path-lab  "$restored" "$IMAGE" 472 /var/lib/grafana
test "$(systemctl --user is-active rh-$restored.service)" = inactive
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$restored"
ready rh-dashboard-restore
offline rh-dashboard-restore "$restored_vol"
verify_dashboard rh-dashboard-restore p8cd-before 'Before Snapshot'
absent_dashboard rh-dashboard-restore p8cd-after
verify_dashboard rh-dashboard-source p8cd-after 'After Snapshot'
echo P8CD_SIXTH_REAL_GRAFANA_SQLITE_INDEPENDENT_DASHBOARD_RESTORE_PASS
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$restored"
podman volume exists "$restored_vol"
verify_dashboard rh-dashboard-source p8cd-after 'After Snapshot'
echo P8CD_RESTORED_GRAFANA_REMOVAL_LEAVES_SOURCE_AND_DATA_PASS
