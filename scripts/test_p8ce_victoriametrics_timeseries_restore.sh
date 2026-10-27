#!/usr/bin/env bash
# Seventh genuinely distinct stateful OCI app: real time-series ingestion,
# stopped snapshot and independent native Rust Quadlet restore.
# Only disposable GitHub Actions rootless Podman.
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
source_slug=timeseries-source
restore_slug=timeseries-restore
source_vol=rh-timeseries-source-data
restore_vol=rh-timeseries-restore-data
test ! -e "$QD/rh-$source_slug.container"
test ! -e "$QD/rh-$restore_slug.container"
! podman volume exists "$source_vol"
! podman volume exists "$restore_vol"
WORK="$(mktemp -d "$(printenv RUNNER_TEMP)/p8ce-series-XXXXXX")"
chmod 0700 "$WORK"
cleanup() {
 HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$restore_slug" >/dev/null 2>&1 || true
 HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$source_slug" >/dev/null 2>&1 || true
 podman volume rm "$restore_vol" "$source_vol" >/dev/null 2>&1 || true
 rm -rf "$WORK"
}
trap cleanup EXIT

# Official upstream unchanged image; never publish a port or overwrite CMD.
podman pull --quiet docker.io/victoriametrics/victoria-metrics:v1.155.0
IMAGE="$(podman image inspect docker.io/victoriametrics/victoria-metrics:v1.155.0 --format '{{index .RepoDigests 0}}')"
[[ "$IMAGE" == docker.io/victoriametrics/victoria-metrics@sha256:* ]]
podman image exists "$IMAGE"
# Python is an ephemeral read-only HTTP probe INSIDE each *network=none*
# container network namespace, not an external bridge/proxy or new service.
podman pull --quiet docker.io/library/python:3.12-alpine
PY_IMAGE="$(podman image inspect docker.io/library/python:3.12-alpine --format '{{index .RepoDigests 0}}')"
[[ "$PY_IMAGE" == docker.io/library/python@sha256:* ]]
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-path-lab  "$source_slug" "$IMAGE" 10001 /victoria-metrics-data
test "$(systemctl --user is-active rh-$source_slug.service)" = inactive
test "$(stat -c %a "$QD/rh-$source_slug.container")" = 600
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$source_slug"
assert_offline() {
 test "$(podman inspect "$1" --format '{{.HostConfig.NetworkMode}}')" = none
 test "$(podman inspect "$1" --format '{{.HostConfig.ReadonlyRootfs}}')" = true
 test "$(podman inspect "$1" --format '{{.Config.User}}')" = 10001
 test "$(podman inspect "$1" --format '{{range .Mounts}}{{if eq .Destination "/victoria-metrics-data"}}{{.Name}}{{end}}{{end}}')" = "$2"
}
tsdb_http() {
 local container="$1" operation="$2" phase="$3"
 podman run --rm --network "container:$container" --user 65534   --read-only --cap-drop ALL --security-opt no-new-privileges   --pids-limit 32 --memory 96m "$PY_IMAGE" python -B -c '
import json,sys,time,urllib.request,urllib.parse
op,phase=sys.argv[1:3]
base="http://127.0.0.1:8428"
metric="p8ce_fixture_value"
query=metric+"{phase=\""+phase+"\"}"
value=41 if phase=="before" else 99
if op=="write":
 # The official engine intentionally hides recently ingested points for
 # 30 seconds and buffers fresh samples before they become queryable.
 # Prometheus uses MILLIS timestamps. This is a disposable test-only flush.
 timestamp_ms=int((time.time()-90)*1000)
 data=(metric+"{phase="+chr(34)+phase+chr(34)+"} "+str(value)+" "+str(timestamp_ms)+chr(10)).encode()
 request=urllib.request.Request(base+"/api/v1/import/prometheus",data=data,method="POST")
 with urllib.request.urlopen(request,timeout=7) as response:
  assert response.status in (200,204),response.status
 with urllib.request.urlopen(base+"/internal/force_flush",timeout=7) as response:
  assert response.status==200,response.status
expected=[] if op=="absent" else [value]
for attempt in range(90):
 try:
  with urllib.request.urlopen(base+"/api/v1/query?"+urllib.parse.urlencode({"query":query}),timeout=5) as response:
   result=json.load(response)["data"]["result"]
  observed=[int(float(series["value"][1])) for series in result]
  if observed==expected:
   print("P8CE_REAL_TSDB_"+op.upper()+"_"+phase.upper()+"_PASS")
   break
 except Exception as exc:
  if attempt==89:raise
 time.sleep(.15)
else:
 raise SystemExit("real VictoriaMetrics query did not equal expected "+str(expected)+": "+str(observed))
' "$operation" "$phase"
}
ready() {
 for _ in $(seq 1 100); do
  if podman run --rm --network "container:$1" --user 65534 --read-only    --cap-drop ALL --security-opt no-new-privileges --pids-limit 32    --memory 96m "$PY_IMAGE" python -B -c '
import urllib.request,sys
try:
 with urllib.request.urlopen("http://127.0.0.1:8428/health",timeout=2) as r:
  sys.exit(0 if r.status==200 else 1)
except Exception:sys.exit(1)' >/dev/null 2>&1; then return 0; fi
  sleep .2
 done
 podman logs "$1" 2>&1 | tail -35 >&2 || true
 return 1
}
ready rh-timeseries-source
assert_offline rh-timeseries-source "$source_vol"
tsdb_http rh-timeseries-source write before
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --stop-lab "$source_slug"
! podman ps --format '{{.Names}}' | grep -Fx rh-timeseries-source
umask 077
ARCHIVE="$WORK/victoriametrics-private.tar"
podman volume export --output "$ARCHIVE" "$source_vol"
chmod 0600 "$ARCHIVE"
test "$(stat -c %a "$ARCHIVE")" = 600
test -s "$ARCHIVE"
tar -tf "$ARCHIVE" > "$WORK/volume-manifest.txt"
test "$(wc -l < "$WORK/volume-manifest.txt")" -ge 2
SHA="$(sha256sum "$ARCHIVE" | cut -d' ' -f1)"
[[ "$SHA" =~ ^[0-9a-f]{64}$ ]]
echo "$SHA  $ARCHIVE" | sha256sum -c
cp "$ARCHIVE" "$WORK/bad.tar"
printf CORRUPTED >> "$WORK/bad.tar"
if echo "$SHA  $WORK/bad.tar" | sha256sum -c >/dev/null 2>&1; then exit 1; fi
echo P8CE_REAL_VICTORIAMETRICS_STOPPED_VOLUME_SHA_TAMPER_REFUSED_PASS
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$source_slug"
ready rh-timeseries-source
tsdb_http rh-timeseries-source get before
tsdb_http rh-timeseries-source write after
echo P8CE_PRIMARY_TSDB_POST_CHECKPOINT_SECOND_SERIES_PASS

! podman volume exists "$restore_vol"
podman volume create "$restore_vol" >/dev/null
echo "$SHA  $ARCHIVE" | sha256sum -c
podman volume import "$restore_vol" "$ARCHIVE"
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --install-path-lab  "$restore_slug" "$IMAGE" 10001 /victoria-metrics-data
test "$(systemctl --user is-active rh-$restore_slug.service)" = inactive
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --start-lab "$restore_slug"
ready rh-timeseries-restore
assert_offline rh-timeseries-restore "$restore_vol"
tsdb_http rh-timeseries-restore get before
tsdb_http rh-timeseries-restore absent after
tsdb_http rh-timeseries-source get after
echo P8CE_SEVENTH_REAL_TSDB_INDEPENDENT_METRICS_CHECKPOINT_RESTORE_PASS
HARS_DISPOSABLE_OCI_LAB=1 "$BIN" --remove-lab "$restore_slug"
podman volume exists "$restore_vol"
tsdb_http rh-timeseries-source get after
echo P8CE_RESTORED_TIMESERIES_APP_REMOVAL_PRESERVES_SOURCE_PASS
