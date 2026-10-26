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
using the official immutable compressed QCOW2 SHA-256. The pinned real HAOS KVM CI passed for P8ac; see exact immutable evidence below.

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

## P8ad — real guest Supervisor/Core CLI readiness probe (genuine CI VERIFIED)

Native Rust now keeps a bounded QEMU guest process and drives its actual HAOS serial shell. The driver sends root login then the official 'ha supervisor info --no-progress --raw-json' and 'ha core info --no-progress --raw-json' read-only CLI calls. Both must return explicit result=ok independently, with retries and a strict boot deadline. The complete console transcript and tokens are neither printed nor persisted; only two boolean proof fields are exposed. Command and image verification are lab-only and cannot create an authorization for production. It validates CLI API responsiveness, not Core HTTP event streaming, Apps running, persistent disk or USB/radio cutover. The pinned KVM workflow p8ad-rust-health.yml must pass at the exact SHA before this scope is marked verified.

## P8ae — persistent Rust-managed guest COW disk (genuine CI VERIFIED)

Rust now provisions a fixed owner-private qcow2 guest-overlay image using the official qemu-img binary with explicit QCOW2 backing pinned to the source image. A nonblocking Linux flock prevents dual VM orchestrators. Rust boots official HAOS twice without -snapshot, writes an invented share marker and syncs inside first guest, forcefully terminates QEMU, boots a second independent QEMU process using the **same overlay** and compares a checksum of the marker read from guest. The original base image hash is checked before and after. This directly exercises persistent guest configuration and crash recovery while preserving the previously certified ephemeral boot and Supervisor CLI health paths. Tests are lab-only: no real Apps data, no physical radio ownership, no host migration. Until real GitHub CI at exact head passes, this is code only.


## P8af: native Rust manager restart recovery (genuine CI VERIFIED)

P8ae proved two QEMU boots from a single Rust manager. New P8af ensures a
SECOND independent Rust process can reattach to the same private qcow2 guest
disk after the first Rust manager quits. It checks owner/mode/single link,
no symlink, exact backing path and qcow2 format using official qemu-img,
holds a cross-process Linux flock to deny dual writers, and proves the
fake HAOS share marker with two genuine QEMU guests in pinned KVM CI.
No running-process adoption, auto-start system service or radio takeover
is implied. Until exact GitHub CI and merge, this is unverified code.
Production remains 0/15.

## Immutable verified native Rust host / official HAOS QEMU evidence (26 October)

- P8ac: official HAOS 18.3 boots under a real native Rust QEMU process. PR #103, HEAD 29f9a45db72e4ef69aa0abed00dda17483d7133e, 18/18 checks, GitHub 40243098816, merged ac76a85dbeadc1456968e5912eb7ddf32c9ba2a3.
- P8ad: official Supervisor and Core CLI health each responded validly from the real guest. PR #104, HEAD 73f91ea3330adfa642e40c0854a001a4eabf206d, 11/11 checks, GitHub 40243728521, merged 230a5964090af47d3a5d06b83c89f8527d868c1c.
- P8ae: persistent private COW QEMU guest file survived forcefully terminated guest and new real official HAOS boot. PR #105, HEAD 4534b23a4b2f0913430251dfe4b32d75fa788c94, 12/12 checks, GitHub 40247000475, merged 45811bcefa791544d7a016dc331d140fc585e120.
- P8af: two INDEPENDENT Rust manager invocations; second safely acquired an exclusive owner lease and recovered first-run fictional file through same qcow2, concurrent owner denied. PR #106, HEAD 267e0fb0e287bb0072a0d933dc6365526f64e4e3, 13/13 checks, GitHub 40247834775, merged d8c5bee817c7b7e8ef5ea6af24d62d366fbe610a.

Machine source of truth: config/haos-rust-guest-evidence.json. These scope-limited KVM proofs MUST NOT be relabeled full host certification. They do not prove an installable long-running guest supervisor daemon, authenticated Core HTTP access, full Apps service lifecycle, household cutover, exclusive hardware ownership, encrypted backing/secret migration or system-level rollback. 0/15 production gates. Reuse the CI history; do not repeat these tests as new evidence.

## P8ag: first continuous Rust guest host (exact KVM CI pending)

An isolated native Rust process now supervises the official HAOS guest
without exiting at the boot marker. It retains the exclusive lease on the
owner-private existing COW disk, starts QEMU with no serial logging or USB
device passthrough, exposes only a Unix socket inside the private workspace,
attempts guest ACPI shutdown, reaps the QEMU child, fsyncs the overlay and
reverifies the immutable official source image. A new Rust lab binary
haos_guest_service_lab provides the opt-in entrypoint. No systemd unit is
installed, no household service is modified and no production readiness
is claimed. This verifies neither the long-term health of Apps nor radio
ownership, authenticated remote API, backups or 72-hour endurance.
