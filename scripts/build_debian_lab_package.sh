#!/usr/bin/env bash
# P12a LAB ONLY: rootless deterministic Debian packaging for a read-only Rust CLI.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
[[ "$(id -u)" != 0 ]] || { echo 'Do not build as root' >&2; exit 2; }
[[ "$#" == 2 ]] || { echo 'Usage: build_debian_lab_package.sh VERSION OUTDIR' >&2; exit 2; }
VERSION="$1"
OUTDIR="$2"
case "$VERSION" in 0.1.0-1|0.1.1-1) ;; *) echo 'Invalid lab-only version' >&2; exit 2;; esac
[[ "$(dpkg --print-architecture)" == amd64 ]] || { echo 'amd64-only lab'; exit 2; }
command -v dpkg-deb >/dev/null
command -v cargo >/dev/null
export SOURCE_DATE_EPOCH
SOURCE_DATE_EPOCH="$(git log -1 --format=%ct)"
[[ "$SOURCE_DATE_EPOCH" =~ ^[0-9]+$ ]]
cargo build --release --locked -p ha-rust-bridge
BIN="$ROOT/target/release/ha-rust-bridge"
[[ -x "$BIN" && ! -L "$BIN" ]] || { echo 'compiled CLI missing' >&2; exit 3; }
mkdir -p "$OUTDIR"
STAGE="$(mktemp -d)"
trap 'rm -rf -- "$STAGE"' EXIT
install -d -m 0755 "$STAGE/DEBIAN" "$STAGE/usr/bin" "$STAGE/usr/share/rusty-home-lab"
install -m 0755 "$BIN" "$STAGE/usr/bin/rusty-home-lab"
printf '%s\n' "$VERSION" > "$STAGE/usr/share/rusty-home-lab/VERSION"
cat > "$STAGE/DEBIAN/control" <<EOF
Package: rusty-home-lab
Version: $VERSION
Architecture: amd64
Maintainer: Rusty Home CI laboratory <lab@example.invalid>
Section: utils
Priority: optional
Depends: libc6 (>= 2.39)
Description: Rusty Home read-only prototype (LAB ONLY, no production support)
 Experimental read-only Docker/HAOS snapshot inspector.
 Does not install a system service, change devices or migrate Home Assistant.
EOF
TARGET="$OUTDIR/rusty-home-lab_${VERSION}_amd64.deb"
dpkg-deb --root-owner-group --build "$STAGE" "$TARGET" >/dev/null
dpkg-deb --field "$TARGET" Version | grep -Fx "$VERSION" >/dev/null
sha256sum "$TARGET"
echo "P12A_DEB_BUILT_LAB_ONLY"
