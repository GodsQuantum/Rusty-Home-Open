#!/usr/bin/env bash
# P12b CI-only ephemeral OpenPGP signed APT package feed; never production.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
[[ "$(id -u)" != 0 ]] || { echo 'Root build is forbidden'; exit 2; }
for tool in gpg dpkg-scanpackages apt-ftparchive docker dpkg-deb; do
  command -v "$tool" >/dev/null
done
IMAGE='ubuntu@sha256:68ee22004a9109b44973313b02ff7b1de250f1de0f5ecb628d73f579aaff33ac'
mkdir -p "$ROOT/runtime"
WORK="$(mktemp -d "$ROOT/runtime/rusty-p12b-XXXXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT
chmod 0755 "$WORK"
mkdir -p "$WORK/packages" "$WORK/repo/pool" "$WORK/bad/pool"
bash scripts/build_debian_lab_package.sh 0.1.0-1 "$WORK/packages"
bash scripts/build_debian_lab_package.sh 0.1.1-1 "$WORK/packages"
# Distribute the genuine native Rust HAOS guest manager alongside the
# previously signed synthetic reader without merging their package names.
bash scripts/build_guest_debian_package_lab.sh 0.1.0-1 "$WORK/packages"
bash scripts/build_guest_debian_package_lab.sh 0.1.1-1 "$WORK/packages"
cp "$WORK/packages/"*.deb "$WORK/repo/pool/"
chmod 0755 "$WORK/repo" "$WORK/repo/pool"
chmod 0644 "$WORK/repo/pool/"*.deb
(
  cd "$WORK/repo"
  dpkg-scanpackages --multiversion pool /dev/null > Packages
  gzip -n -c Packages > Packages.gz
  apt-ftparchive -o APT::FTPArchive::Release::Origin='Rusty Home Laboratory' \
    -o APT::FTPArchive::Release::Label='Rusty Home CI only' \
    -o APT::FTPArchive::Release::Suite='lab' \
    -o APT::FTPArchive::Release::Codename='lab' \
    release . > Release
)
# Both native guest versions must actually be in the signed index.
test "$(grep -c '^Package: rusty-home-guest-lab$' "$WORK/repo/Packages")" -eq 2
grep -q '^Version: 0.1.0-1$' "$WORK/repo/Packages"
grep -q '^Version: 0.1.1-1$' "$WORK/repo/Packages"
echo P8AL_NATIVE_RUST_GUEST_BOTH_VERSIONS_IN_SIGNED_INDEX_PASS
chmod 0644 "$WORK/repo/"{Packages,Packages.gz,Release}
GNUPGHOME="$WORK/gnupg"
export GNUPGHOME
install -d -m 0700 "$GNUPGHOME"
gpg --batch --quiet --passphrase '' --quick-generate-key \
  'Rusty Home Discarded CI Lab Key <apt-test@example.invalid>' ed25519 sign 1d
gpg --batch --yes --quiet --export > "$WORK/repo/lab-apt-public.gpg"
gpg --batch --yes --quiet --digest-algo SHA256 \
  --clearsign -o "$WORK/repo/InRelease" "$WORK/repo/Release"
gpg --batch --yes --quiet --digest-algo SHA256 \
  --detach-sign -o "$WORK/repo/Release.gpg" "$WORK/repo/Release"
chmod 0644 "$WORK/repo/"{lab-apt-public.gpg,InRelease,Release.gpg}
# Bad repo changes a signed Release record after signing. It must NOT be used.
cp -a "$WORK/repo/." "$WORK/bad/"
printf 'UNSIGNED_TAMPERING\n' >> "$WORK/bad/InRelease"
chmod -R a+rX "$WORK/repo" "$WORK/bad"
echo P12B_EPHEMERAL_SIGNED_REPO_PREPARED

# The host pulls a pinned Linux runtime before offline evaluation.
docker pull "$IMAGE" >/dev/null
docker run --rm --name "rusty-home-lab-apt-$RANDOM" --network none \
  --cap-drop ALL --security-opt no-new-privileges \
  --pids-limit 128 --memory 750m --cpus 1 \
  --mount "type=bind,src=$WORK/repo,dst=/repo,readonly" \
  --mount "type=bind,src=$WORK/bad,dst=/bad,readonly" \
  --mount "type=bind,src=$ROOT/fixtures/docker-ps.jsonl,dst=/fixture.jsonl,readonly" \
  "$IMAGE" sh -euc '
    install -d -m 0755 /etc/apt/keyrings
    cp /repo/lab-apt-public.gpg /etc/apt/keyrings/rusty-home-lab.gpg
    printf "deb [signed-by=/etc/apt/keyrings/rusty-home-lab.gpg] file:///repo ./\n" > /tmp/good.list
    apt_ci() {
      apt-get -o Dir::Etc::sourcelist=/tmp/good.list \
        -o Dir::Etc::sourceparts=- -o APT::Sandbox::User=root \
        -o APT::Get::List-Cleanup=0 "$@"
    }
    apt_ci update -qq
    # The offline APT client accepts this index only under our ephemeral
    # signed-by key. Native runtime is not started/installed in this container:
    # its full real-dpkg lifecycle was independently verified in P8ak.
    apt-cache -o Dir::Etc::sourcelist=/tmp/good.list       -o Dir::Etc::sourceparts=- -o APT::Sandbox::User=root       show rusty-home-guest-lab > /tmp/guest-signed-index
    grep -q "^Package: rusty-home-guest-lab$" /tmp/guest-signed-index
    grep -q "^Version: 0.1.0-1$" /tmp/guest-signed-index
    grep -q "^Version: 0.1.1-1$" /tmp/guest-signed-index
    echo P8AL_SIGNED_OFFLINE_APT_DISCOVERS_NATIVE_RUST_GUEST_PASS

    apt_ci install -y -qq rusty-home-lab=0.1.0-1
    test "$(dpkg-query -W -f="\${Version}" rusty-home-lab)" = 0.1.0-1
    /usr/bin/rusty-home-lab --fixture /fixture.jsonl --host offline >/tmp/readonly.json
    grep -q "\"read_only\":true" /tmp/readonly.json
    apt_ci install -y -qq rusty-home-lab=0.1.1-1
    test "$(dpkg-query -W -f="\${Version}" rusty-home-lab)" = 0.1.1-1
    /usr/bin/rusty-home-lab --fixture /fixture.jsonl --host offline >/tmp/readonly.json
    grep -q "\"read_only\":true" /tmp/readonly.json
    echo P12B_SIGNED_APT_INSTALL_AND_UPGRADE_PASS
    printf "deb [signed-by=/etc/apt/keyrings/rusty-home-lab.gpg] file:///bad ./\n" > /tmp/bad.list
    install -d -m 0755 /tmp/badlists/partial
    if apt-get -o Dir::Etc::sourcelist=/tmp/bad.list \
      -o Dir::Etc::sourceparts=- -o APT::Sandbox::User=root \
      -o Dir::State::lists=/tmp/badlists \
      update >/tmp/tamper-out 2>&1; then
      echo "SECURITY FAILURE: tampered APT InRelease accepted" >&2
      exit 83
    fi
    # Failure is relevant only when APT detects a broken signature;
    # unrelated file://, apt-list or network errors are NOT accepted.
    if ! grep -Eiq "clearsigned|signature|badsig|not signed|gpg error" /tmp/tamper-out; then
      echo "SECURITY FAILURE: signature error was not the observed cause" >&2
      exit 84
    fi
    test "$(dpkg-query -W -f="\${Version}" rusty-home-lab)" = 0.1.1-1
    echo P12B_TAMPERED_APT_SIGNATURE_REJECTED_PASS
  '
echo P12B_SIGNED_APT_OFFLINE_END_TO_END_PASS
