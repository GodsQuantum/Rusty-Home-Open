#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo test --locked -p hars-supervisor-apps --test product_app_manifest
cargo build --release --locked --target x86_64-unknown-linux-musl -p ha-rust-bridge

BIN="$ROOT/target/x86_64-unknown-linux-musl/release/ha-rust-bridge"
CONTEXT="$(mktemp -d)"
IMAGE="rusty-home-product-ci:local"
CONTAINER=""
cleanup() {
  if [[ -n "$CONTAINER" ]]; then docker rm -f "$CONTAINER" >/dev/null 2>&1 || true; fi
  docker image rm -f "$IMAGE" >/dev/null 2>&1 || true
  rm -rf "$CONTEXT"
}
trap cleanup EXIT

cp "$ROOT/packaging/product-app/rusty_home/Dockerfile" "$CONTEXT/Dockerfile"
install -m 0555 "$BIN" "$CONTEXT/rusty-home"
# Rootless scratch image must actually ship the first-party MIT text.
install -m 0644 "$ROOT/LICENSE" "$CONTEXT/LICENSE"
docker build -q -t "$IMAGE" "$CONTEXT"

CONTAINER="$(docker run --detach --rm --read-only --cap-drop=ALL \
  --security-opt=no-new-privileges --pids-limit=96 --memory=512m \
  --publish 127.0.0.1:18795:8099 \
  --env SUPERVISOR_TOKEN=disposable_ci_token_only "$IMAGE")"
for _ in {1..30}; do
  if curl -s -o /dev/null --max-time 1 http://127.0.0.1:18795/health; then break; fi
  sleep 0.1
done
[[ "$(docker inspect --format '{{.State.Running}}' "$CONTAINER")" == "true" ]]
[[ "$(docker inspect --format '{{.Config.User}}' "$CONTAINER")" == "1000:1000" ]]
docker cp "$CONTAINER:/usr/share/licenses/rusty-home/LICENSE" - |
  tar -xO | cmp - "$ROOT/LICENSE" || {
    echo "FAIL: distributed Rusty Home container lacks exact MIT notice" >&2
    exit 1
  }
for route in / /health /api/apps /api/readiness; do
  status="$(curl -s -o /dev/null -w '%{http_code}' --max-time 3 \
    -H 'X-Remote-User-Id: admin' http://127.0.0.1:18795"$route")"
  [[ "$status" == "403" ]] || {
    echo "FAIL: official ingress security allowed unauthorized host GET $route: $status"
    exit 1
  }
done
status="$(curl -s -o /dev/null -w '%{http_code}' --max-time 3 \
  -X POST -H 'X-Remote-User-Id: admin' http://127.0.0.1:18795/api/apps)"
[[ "$status" == "403" ]] || { echo "FAIL: unauthorized POST $status"; exit 1; }
echo "PASS: real statically-linked product App starts as nonroot, source-authenticated ingress, no host privileges"
