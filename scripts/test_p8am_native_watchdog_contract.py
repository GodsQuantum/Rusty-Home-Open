#!/usr/bin/env python3
from pathlib import Path
b=Path(__file__).resolve().parent.parent
unit=(b/"packaging/systemd/rusty-home-haos-lab.service").read_text()
source=(b/"crates/native-hub/src/haos_guest.rs").read_text()
toml=(b/"crates/native-hub/Cargo.toml").read_text()
lock=(b/"Cargo.lock").read_text()
smoke=(b/"scripts/test_p8aj_real_systemd_guest.sh").read_text()
wf=(b/".github/workflows/p8aj-real-systemd.yml").read_text()
for line in ("Type=notify","NotifyAccess=main","WatchdogSec=30s",
             "Restart=on-failure","KillMode=control-group"):
 assert line in unit,line
assert "sd-notify = \"=0.5.0\"" in toml
assert 'name = "sd-notify"' in lock and 'version = "0.5.0"' in lock
for marker in ("NotifyState::Ready","NotifyState::Watchdog","NotifyState::Stopping",
               "sd_notify::watchdog_enabled","local_qmp_responsive",
               "stop_guest(&mut child)"):
 assert marker in source,marker
for marker in ("WatchdogTimestampMonotonic",
               "P8AM_GENUINE_SYSTEMD_NOTIFY_QMP_READY_AND_WATCHDOG_BEAT_PASS"):
 assert marker in smoke,marker
assert "test_p8am_native_watchdog_contract.py" in wf
assert "[Install]" not in [x.strip() for x in unit.splitlines() if not x.lstrip().startswith("#")]
assert "hostfwd=" not in unit
print("P8AM_RUST_NATIVE_SYSTEMD_QMP_READY_HEARTBEAT_CONTRACT_PASS")
