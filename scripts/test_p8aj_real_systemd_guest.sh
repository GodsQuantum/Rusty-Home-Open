#!/usr/bin/env bash
# LAB ONLY: genuine systemd starts, stops, and restarts native Rust + official HAOS.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[[ "${GITHUB_ACTIONS:-}" == "true" && -n "${RUNNER_TEMP:-}" ]] || exit 2
[[ "$(id -u)" != "0" && -c /dev/kvm ]] || exit 2
unit=rusty-home-haos-lab.service
[[ ! -e /run/systemd/system/$unit ]] || exit 2
! id rusty-home-lab >/dev/null 2>&1 || exit 2
[[ -s "$RUNNER_TEMP/haos-native-rust.qcow2" ]]
getent group kvm >/dev/null
sudo useradd --system --no-create-home --home-dir /var/lib/rusty-home-lab   --shell /usr/sbin/nologin --gid kvm rusty-home-lab
cleanup() {
  sudo systemctl stop "$unit" >/dev/null 2>&1 || true
  sudo rm -f "/run/systemd/system/$unit"
  sudo systemctl daemon-reload >/dev/null 2>&1 || true
  sudo rm -f /usr/libexec/rusty-home/haos_guest_service_lab
  sudo rm -f /run/rusty-home-lab/haos-guest.env
  sudo userdel rusty-home-lab >/dev/null 2>&1 || true
}
trap cleanup EXIT
sudo install -d -m 0700 -o rusty-home-lab -g kvm /var/lib/rusty-home-lab
sudo install -d -m 0700 /run/rusty-home-lab
sudo install -d -m 0755 /usr/libexec/rusty-home
sudo install -m 0755 "$ROOT/target/debug/haos_guest_service_lab"   /usr/libexec/rusty-home/haos_guest_service_lab
sudo install -m 0600 -o rusty-home-lab -g kvm   "$RUNNER_TEMP/haos-native-rust.qcow2" /var/lib/rusty-home-lab/haos-base.qcow2
sudo -u rusty-home-lab qemu-img create -f qcow2 -F qcow2   -b /var/lib/rusty-home-lab/haos-base.qcow2   /var/lib/rusty-home-lab/guest-overlay.qcow2
sudo chmod 0600 /var/lib/rusty-home-lab/guest-overlay.qcow2
base_sha="$(sudo sha256sum /var/lib/rusty-home-lab/haos-base.qcow2 | cut -d' ' -f1)"
printf 'HAOS_BASE_SHA256=%s\n' "$base_sha" | sudo tee /run/rusty-home-lab/haos-guest.env >/dev/null
sudo chmod 0600 /run/rusty-home-lab/haos-guest.env
sudo install -m 0644 "$ROOT/packaging/systemd/$unit" "/run/systemd/system/$unit"
sudo systemd-analyze verify "/run/systemd/system/$unit"
sudo systemctl daemon-reload
[[ "$(sudo systemctl is-enabled "$unit")" == "static" ]]
overlay_inode="$(sudo stat -c '%i' /var/lib/rusty-home-lab/guest-overlay.qcow2)"
for round in 1 2; do
  sudo systemctl start "$unit"
  for _ in $(seq 1 150); do
    [[ "$(sudo systemctl is-active "$unit")" == "active" ]] || exit 1
    sudo test -S /var/lib/rusty-home-lab/rusty-haos-lab-qmp.sock && break
    sleep 0.2
  done
  sudo test -S /var/lib/rusty-home-lab/rusty-haos-lab-qmp.sock
  # Native Type=notify means systemctl start returned ONLY after Rust
  # received a real private QMP greeting; not merely process exec().
  [[ "$(sudo systemctl show -P Type "$unit")" == "notify" ]]
  [[ "$(sudo systemctl show -P NotifyAccess "$unit")" == "main" ]]
  [[ "$(sudo systemctl show -P WatchdogUSec "$unit")" != "0" ]]
  stamp_before="$(sudo systemctl show -P WatchdogTimestampMonotonic "$unit")"
  sleep 12
  stamp_after="$(sudo systemctl show -P WatchdogTimestampMonotonic "$unit")"
  [[ "$stamp_before" =~ ^[0-9]+$ && "$stamp_after" =~ ^[0-9]+$ ]]
  (( stamp_after > stamp_before ))
  echo "P8AM_GENUINE_SYSTEMD_NOTIFY_QMP_READY_AND_WATCHDOG_BEAT_PASS round=$round"
  main_pid="$(sudo systemctl show -P MainPID "$unit")"
  [[ "$main_pid" =~ ^[0-9]+$ && "$main_pid" != "0" ]]
  sudo pgrep -u rusty-home-lab -f qemu-system-x86_64 >/dev/null
  [[ "$(sudo stat -c '%i' /var/lib/rusty-home-lab/guest-overlay.qcow2)" == "$overlay_inode" ]]
  echo "P8AJ_REAL_SYSTEMD_ACTIVE_QEMU_LOCKED_PASS round=$round"
  sudo systemctl stop "$unit"
  [[ "$(sudo systemctl is-active "$unit")" == "inactive" ]]
  ! sudo test -e /var/lib/rusty-home-lab/rusty-haos-lab-qmp.sock
  ! sudo pgrep -u rusty-home-lab -f qemu-system-x86_64 >/dev/null
  [[ "$(sudo systemctl show -P Result "$unit")" == "success" ]]
  echo "P8AJ_SYSTEMD_STOP_REAPS_QEMU_PASS round=$round"
done
if [[ "$(printenv HARS_CRASH_RECOVERY_LAB || true)" == 1 ]]; then
  # Genuine QEMU kill permitted only inside guarded disposable GitHub runner.
  # Rust must exit nonzero, systemd restarts it and reattaches existing QCOW2.
  # A fully stopped static unit may already have been unloaded by systemd.
  # systemctl start re-loads it; reset-failed on an unloaded unit exits
  # nonzero and prevented the actual SIGKILL recovery scenario from running.
  sudo systemctl start "$unit"
  rust_before="$(sudo systemctl show -P MainPID "$unit")"
  [[ "$rust_before" =~ ^[0-9]+$ && "$rust_before" != "0" ]]
  qemu_pid="$(sudo pgrep -u rusty-home-lab -f '^/usr/bin/qemu-system-x86_64' | head -1)"
  [[ "$qemu_pid" =~ ^[0-9]+$ ]]
  sudo kill -KILL "$qemu_pid"
  recovered=0
  for attempt in $(seq 1 180); do
    rust_after="$(sudo systemctl show -P MainPID "$unit")"
    restarts="$(sudo systemctl show -P NRestarts "$unit")"
    if [[ "$rust_after" =~ ^[0-9]+$ && "$rust_after" != 0 &&
          "$rust_after" != "$rust_before" && "$restarts" =~ ^[0-9]+$ ]] &&
          (( restarts > 0 )) &&
          [[ "$(sudo systemctl is-active "$unit")" == "active" ]] &&
          sudo test -S /var/lib/rusty-home-lab/rusty-haos-lab-qmp.sock; then
      recovered=1
      break
    fi
    sleep 0.4
  done
  [[ "$recovered" == 1 ]]
  [[ "$(sudo stat -c '%i' /var/lib/rusty-home-lab/guest-overlay.qcow2)" == "$overlay_inode" ]]
  echo P8AN_REAL_QEMU_CRASH_SYSTEMD_AUTO_RESTART_SAME_DISK_PASS
  sudo systemctl stop "$unit"
  [[ "$(sudo systemctl is-active "$unit")" == "inactive" ]]
  ! sudo pgrep -u rusty-home-lab -f qemu-system-x86_64 >/dev/null
  ! sudo test -e /var/lib/rusty-home-lab/rusty-haos-lab-qmp.sock
  echo P8AN_RESTARTED_GUEST_CLEAN_STOP_NO_ORPHANS_PASS
fi
echo "$base_sha  /var/lib/rusty-home-lab/haos-base.qcow2" | sudo sha256sum -c
[[ "$(sudo stat -c '%i' /var/lib/rusty-home-lab/guest-overlay.qcow2)" == "$overlay_inode" ]]
echo P8AJ_REAL_SYSTEMD_RESTART_SAME_OVERLAY_AND_PINNED_SOURCE_PASS
