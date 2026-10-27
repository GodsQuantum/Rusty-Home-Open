#!/usr/bin/env python3
"""P8du static scope and real official QEMU JUnit acceptance, not certification."""
import argparse
from pathlib import Path
from xml.etree import ElementTree
root = Path(__file__).resolve().parents[1]
workflow = (root/".github/workflows/p8du-two-real-haos-app-atomic.yml").read_text()
guest = (root/"scripts/p8du_two_real_apps_qemu_pytest.py").read_text()
native = (root/"scripts/test_p8du_two_real_apps_native_pair.py").read_text()
for value in (
    "runs-on: ubuntu-24.04", "haos_ova-18.3.qcow2.xz",
    "fae6a728768cc10aff60d4820bfcd40d64cd77fab82c8bd92af13b3d9d414090",
    "90fa213c3441e585d5a60387e9244d0ed38efaf8",
    "-snapshot -accel kvm", "test_hars_real_two_simultaneous_app_only_partial_exports",
    "test_p8du_two_real_apps_native_pair.py",
    "cargo clippy --locked -p hars-interop-moli --all-targets -- -D warnings",
):
    assert value in workflow, value
for value in (
    "core_configurator", "core_mosquitto", "ha apps install ",
    "ha backups new", "ha backups info", "ha backups remove", "ha apps uninstall",
    "18674", "18675", "P8DU_REAL_HAOS_TWO_OFFICIAL_APPS_SIMULTANEOUSLY_INSTALLED_PASS",
):
    assert value in guest, value
for value in (
    "source_boot_id", "app-source-ingest", "app-source-verify",
    "app-source-stage-pair-inactive", "SIGKILL",
    "P8DU_TWO_REAL_HAOS_APP_BYTES_ATOMIC_PRIVATE_RETRY_SOURCE_UNCHANGED_PASS",
):
    assert value in native, value
for banned in ("workflow_dispatch:", "pull_request_target:", "sudo podman",
               "qm stop 100", "pct exec", "192.168."):
    assert banned not in workflow+guest+native, banned
p = argparse.ArgumentParser()
p.add_argument("--junit", type=Path)
args = p.parse_args()
if args.junit:
    assert args.junit.is_file() and args.junit.stat().st_size < 1048576
    cases = ElementTree.parse(args.junit).getroot().findall(".//testcase")
    expected = {"test_start_supervisor", "test_check_supervisor",
                "test_hars_real_two_simultaneous_app_only_partial_exports"}
    assert len(cases) == 3 and {c.get("name") for c in cases} == expected
    assert all(c.find(t) is None for c in cases for t in ("failure", "error", "skipped"))
    print("P8DU_THREE_GENUINE_QEMU_JUNIT_NO_SKIP_PASS")
else:
    print("P8DU_REAL_DUAL_HAOS_APP_ISOLATED_SCOPE_CONTRACT_PASS")

