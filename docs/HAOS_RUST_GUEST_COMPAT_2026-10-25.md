# P8ac: Native Rust managing genuine HAOS compatibility guest

**25 October 2026. NOT_PRODUCTION_READY, 0/15 complete release gates.**

Upstream Home Assistant supports HAOS (includes Supervisor and Apps) and
Home Assistant Container (no Supervisor/Apps). Generic Linux Supervised was
retired in 2025.12. Therefore the simplest compatibility-preserving
architecture is Rust host orchestration of a fully official HAOS guest,
not installing unsupported Supervisor on plain Linux and not pretending
the synthetic Rust native kernel implements every Home Assistant integration.

Native Rust code in crates/native-hub/src/haos_guest.rs and native executable
crates/native-hub/src/bin/haos_guest_lab.rs starts real QEMU with KVM. All
untrusted image paths must be absolute, owned, 0600 regular, no hardlink,
no symlink and no comma-based QEMU option injection. SHA-256 is validated
before and after guest run; both -snapshot and snapshot=on keep disk ephemeral.
QEMU arguments are fixed: actual official HAOS image, OVMF, 3GiB, 2 cores,
user-mode NIC, serial console, disabled monitor, no hostfwd, no mounted
host paths, no physical USB/radio devices. Rust owns, kills and reaps QEMU
after a bounded real HAOS 'homeassistant login:' console boot marker.

CI tests genuine pinned HAOS 18.3 under KVM in a disposable GitHub runner
using the official immutable compressed QCOW2 SHA-256. Until exact GitHub
SHA and real CI success, this is only proposed evidence.

REMAINING: boot marker is NOT Supervisor/Core readiness. Production needs a
persistent guest disk (not -snapshot), scoped and authenticated Core+Supervisor
health APIs, systemd lifecycle, upgrades, rollbacks, isolated guest networking,
safe single-radio host/guest ownership, authenticated UI/write routing,
multi-architecture packaging, data migration and 72h endurance/field testing.
Previous P8p/P8aa/P8ab real backups and return to HAOS are preserved and
must not be repeated as new evidence.

Official docs:
https://contributors.home-assistant.io/os/installation-types/
https://contributors.home-assistant.io/architecture/platform/supervisor/
https://github.com/home-assistant/operating-system

0/15 production gates cleared, no household HAOS, radios or credentials touched.
