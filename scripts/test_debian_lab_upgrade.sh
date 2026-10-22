#!/usr/bin/env bash
# Real dpkg install -> upgrade -> downgrade -> purge, disposable offline Docker.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
[[ "$(id -u)" != 0 ]] || { echo 'Never package from root'; exit 2; }
mkdir -p "$ROOT/runtime"
# Docker-outside-of-Docker can bind the shared Git worktree, not private /tmp.
OUTDIR="$(mktemp -d "$ROOT/runtime/rusty-home-p12-XXXXXX")"
# Archives contain only the synthetic lab binary: readable to rootless Docker.
chmod 0755 "$OUTDIR"
trap 'rm -rf -- "$OUTDIR"' EXIT
bash scripts/build_debian_lab_package.sh 0.1.0-1 "$OUTDIR"
bash scripts/build_debian_lab_package.sh 0.1.1-1 "$OUTDIR"
# Rebuild an identical version from the same Cargo.lock to check archive bytes.
mkdir "$OUTDIR/repro"
bash scripts/build_debian_lab_package.sh 0.1.0-1 "$OUTDIR/repro"
cmp "$OUTDIR/rusty-home-lab_0.1.0-1_amd64.deb" \
    "$OUTDIR/repro/rusty-home-lab_0.1.0-1_amd64.deb"
echo P12A_REPRODUCIBLE_DEBIAN_PACKAGE_PASS
for version in 0.1.0-1 0.1.1-1; do
  FILE="$OUTDIR/rusty-home-lab_${version}_amd64.deb"
  [[ -s "$FILE" ]]
  [[ "$(dpkg-deb --field "$FILE" Version)" == "$version" ]]
  [[ "$(dpkg-deb --field "$FILE" Package)" == "rusty-home-lab" ]]
  ! dpkg-deb --contents "$FILE" | grep -q '/etc/systemd/'
done
# Pull the fixture OS first; the test container itself has no networking.
docker pull ubuntu@sha256:68ee22004a9109b44973313b02ff7b1de250f1de0f5ecb628d73f579aaff33ac >/dev/null
docker run --rm --name "rusty-home-lab-upgrade-$RANDOM" --network none --cap-drop ALL --security-opt no-new-privileges \
  --pids-limit 128 --memory 512m --cpus 1 \
  --mount "type=bind,src=$OUTDIR,dst=/packages,readonly" \
  --mount "type=bind,src=$ROOT/fixtures/docker-ps.jsonl,dst=/fixture.jsonl,readonly" \
  ubuntu@sha256:68ee22004a9109b44973313b02ff7b1de250f1de0f5ecb628d73f579aaff33ac sh -euc '
    check_version() {
      expected="$1"
      actual="$(dpkg-query -W -f="\${Version}" rusty-home-lab)"
      test "$actual" = "$expected"
      test "$(cat /usr/share/rusty-home-lab/VERSION)" = "$expected"
      test -x /usr/bin/rusty-home-lab
      /usr/bin/rusty-home-lab --fixture /fixture.jsonl --host isolated >/tmp/readonly.json
      grep -q "\"read_only\":true" /tmp/readonly.json
    }
    dpkg -i /packages/rusty-home-lab_0.1.0-1_amd64.deb
    check_version 0.1.0-1
    dpkg -i /packages/rusty-home-lab_0.1.1-1_amd64.deb
    check_version 0.1.1-1
    dpkg -i /packages/rusty-home-lab_0.1.0-1_amd64.deb
    check_version 0.1.0-1
    dpkg --purge rusty-home-lab
    test ! -e /usr/bin/rusty-home-lab
    echo P12A_REAL_DPKG_INSTALL_UPGRADE_DOWNGRADE_PURGE_PASS
  '
