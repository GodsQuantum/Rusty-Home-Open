#!/usr/bin/env bash
# Deterministic Debian package for the lab-only native Rust HAOS guest manager.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
[[ "$(id -u)" != "0" && $# -eq 2 ]] || exit 2
VERSION="$1"
OUTDIR="$2"
case "$VERSION" in 0.1.0-1|0.1.1-1) ;; *) echo 'invalid lab version' >&2;exit 2;; esac
[[ "$(dpkg --print-architecture)" == amd64 ]] || exit 2
export SOURCE_DATE_EPOCH
SOURCE_DATE_EPOCH="$(git log -1 --format=%ct)"
cargo build --release --locked -p hars-native-hub --bin haos_guest_service_lab
BIN="$ROOT/target/release/haos_guest_service_lab"
[[ -x "$BIN" && ! -L "$BIN" ]]
mkdir -p "$OUTDIR"
STAGE="$(mktemp -d)"
trap 'rm -rf -- "$STAGE"' EXIT
install -d -m 0755 "$STAGE/DEBIAN" "$STAGE/usr/libexec/rusty-home"  "$STAGE/usr/lib/systemd/system" "$STAGE/usr/lib/sysusers.d"  "$STAGE/usr/share/doc/rusty-home-guest-lab"
install -m 0755 "$BIN" "$STAGE/usr/libexec/rusty-home/haos_guest_service_lab"
install -m 0644 packaging/systemd/rusty-home-haos-lab.service  "$STAGE/usr/lib/systemd/system/rusty-home-haos-lab.service"
printf '%s\n' 'u rusty-home-lab - "Rusty Home isolated HAOS lab" /var/lib/rusty-home-lab /usr/sbin/nologin'  > "$STAGE/usr/lib/sysusers.d/rusty-home-guest-lab.conf"
printf '%s\n' 'LAB ONLY. Service has NO Install section and is not started/enabled.'  > "$STAGE/usr/share/doc/rusty-home-guest-lab/README"
printf '%s\n' "$VERSION" > "$STAGE/usr/share/doc/rusty-home-guest-lab/VERSION"
cat > "$STAGE/DEBIAN/control" <<EOF
Package: rusty-home-guest-lab
Version: $VERSION
Architecture: amd64
Maintainer: Rusty Home CI <lab@example.invalid>
Section: utils
Priority: optional
Depends: libc6 (>= 2.39), systemd, qemu-system-x86, qemu-utils, ovmf
Description: Experimental native Rust HAOS guest manager - disposable lab only
 No automatic start, no public listener and no household deployment.
EOF
cat > "$STAGE/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -eu
if [ "$1" = configure ]; then
  command -v systemd-sysusers >/dev/null 2>&1 ||
    { echo 'systemd-sysusers unavailable' >&2; exit 1; }
  systemd-sysusers /usr/lib/sysusers.d/rusty-home-guest-lab.conf
fi
exit 0
EOF
chmod 0755 "$STAGE/DEBIAN/postinst"
OUT="$OUTDIR/rusty-home-guest-lab_"$VERSION"_amd64.deb"
dpkg-deb --root-owner-group --build "$STAGE" "$OUT" >/dev/null
dpkg-deb --field "$OUT" Version | grep -Fx "$VERSION" >/dev/null
sha256sum "$OUT"
echo P8AK_REPRODUCIBLE_DEBIAN_GUEST_MANAGER_BUILT
