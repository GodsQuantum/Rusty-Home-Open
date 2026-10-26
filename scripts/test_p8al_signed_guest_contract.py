#!/usr/bin/env python3
from pathlib import Path
r=Path(__file__).resolve().parent.parent
script=(r/'scripts/test_signed_apt_lab.sh').read_text()
workflow=(r/'.github/workflows/public-debian-apt.yml').read_text()
for token in ("build_guest_debian_package_lab.sh 0.1.0-1",
 "build_guest_debian_package_lab.sh 0.1.1-1",
 "P8AL_NATIVE_RUST_GUEST_BOTH_VERSIONS_IN_SIGNED_INDEX_PASS",
 "P8AL_SIGNED_OFFLINE_APT_DISCOVERS_NATIVE_RUST_GUEST_PASS",
 "dpkg-scanpackages --multiversion","apt-ftparchive",
 "--clearsign","signed-by=",
 "P12B_TAMPERED_APT_SIGNATURE_REJECTED_PASS"):
 assert token in script,token
for token in ("test_signed_apt_lab.sh","test_p8al_signed_guest_contract.py",
              "signed-apt-negative","ubuntu-24.04"):
 assert token in workflow,token
assert "apt_ci install -y -qq rusty-home-guest-lab" not in script
assert "hostfwd" not in script and "192.168." not in script
print("P8AL_AUTHENTICATED_OFFLINE_NATIVE_RUST_GUEST_APT_INDEX_CONTRACT_PASS")
