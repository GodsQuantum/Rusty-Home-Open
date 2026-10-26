#!/usr/bin/env bash
# Two revisions of genuine Rust guest manager .deb tested on disposable GitHub OS.
set -euo pipefail
[[ "$GITHUB_ACTIONS" == true && -n "$RUNNER_TEMP" ]] || exit 2
[[ "$(id -u)" != 0 ]] || exit 2
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OUTDIR="$(mktemp -d "$RUNNER_TEMP/rusty-guest-debian-XXXXXX")"
cleanup() {
  sudo dpkg --purge rusty-home-guest-lab >/dev/null 2>&1 || true
  sudo systemctl daemon-reload >/dev/null 2>&1 || true
}
trap cleanup EXIT
bash scripts/build_guest_debian_package_lab.sh 0.1.0-1 "$OUTDIR"
bash scripts/build_guest_debian_package_lab.sh 0.1.1-1 "$OUTDIR"
mkdir "$OUTDIR/repro"
bash scripts/build_guest_debian_package_lab.sh 0.1.0-1 "$OUTDIR/repro"
cmp "$OUTDIR/rusty-home-guest-lab_0.1.0-1_amd64.deb"  "$OUTDIR/repro/rusty-home-guest-lab_0.1.0-1_amd64.deb"
echo P8AK_REPRODUCIBLE_NATIVE_RUST_GUEST_DEBIAN_PASS
for v in 0.1.0-1 0.1.1-1; do
  file="$OUTDIR/rusty-home-guest-lab_"$v"_amd64.deb"
  [[ -s "$file" ]]
  [[ "$(dpkg-deb --field "$file" Package)" == rusty-home-guest-lab ]]
  [[ "$(dpkg-deb --field "$file" Version)" == "$v" ]]
  dpkg-deb --contents "$file" | grep -F /usr/libexec/rusty-home/haos_guest_service_lab
  dpkg-deb --contents "$file" | grep -F /usr/lib/systemd/system/rusty-home-haos-lab.service
  dpkg-deb --contents "$file" | grep -F /usr/lib/sysusers.d/rusty-home-guest-lab.conf
  ! dpkg-deb --contents "$file" | grep -E '(etc/systemd|usr/local|var/lib/rusty-home-lab/)'
done
for v in 0.1.0-1 0.1.1-1 0.1.0-1; do
  sudo dpkg -i "$OUTDIR/rusty-home-guest-lab_"$v"_amd64.deb"
  sudo systemctl daemon-reload
  [[ "$(dpkg-query -W rusty-home-guest-lab | awk '{print $2}')" == "$v" ]]
  [[ "$(cat /usr/share/doc/rusty-home-guest-lab/VERSION)" == "$v" ]]
  test -x /usr/libexec/rusty-home/haos_guest_service_lab
  cmp /usr/lib/systemd/system/rusty-home-haos-lab.service     packaging/systemd/rusty-home-haos-lab.service
  id rusty-home-lab >/dev/null
  [[ "$(systemctl is-enabled rusty-home-haos-lab.service)" == static ]]
  [[ "$(systemctl is-active rusty-home-haos-lab.service)" == inactive ]]
  echo "P8AK_ACTUAL_SYSTEMD_GUEST_PACKAGE_DPKG_VERSION_PASS version=$v"
done
sudo dpkg --purge rusty-home-guest-lab
sudo systemctl daemon-reload
! dpkg-query -W rusty-home-guest-lab >/dev/null 2>&1
test ! -e /usr/libexec/rusty-home/haos_guest_service_lab
test ! -e /usr/lib/systemd/system/rusty-home-haos-lab.service
echo P8AK_ACTUAL_DPKG_UPGRADE_DOWNGRADE_PURGE_NO_AUTOSTART_PASS
