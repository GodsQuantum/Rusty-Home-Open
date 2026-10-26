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

## P8ah — fixed localhost-only frontend transport (real KVM CI pending)

The official HAOS 18.3 guest now has an optional local-user-only QEMU
frontend bridge in the Rust guest host service. Only 127.0.0.1:18180
is bound on the host; the guest port is 80 (HAOS uses 80 since
Home Assistant 2026.8). The original mode keeps zero host forwards.
The new mode does not provide credentials, a public interface, proxy
trust, real radio USB pass-through or app control. Rust only detects
the local TCP listener, not that the entire HA Core UI is ready.
A separate real KVM certification at exact feature SHA is required.
Production certification remains 0/15; family HAOS untouched.

## P8ai: genuine frontend response from official HAOS guest (CI pending)

Unlike P8ah, which only proves a host TCP listener, P8ai opens a
read-only HTTP connection from native Rust to the official Home Assistant
Core frontend over the fixed 127.0.0.1:18180 to guest TCP/80 route.
It accepts only successful HTTP 200 or frontend redirection (302/307).
It never logs HTTP response content and sends no passwords or access
tokens. If Core does not start by the bounded disposable CI deadline,
the laboratory fails closed. A fresh official HAOS 18.3 KVM test at the
exact feature SHA is required; no conclusion about real Apps, logged-in
sessions, Core WebSockets or production service is implied.

## P8aj — actual systemd supervision (official KVM CI pending)

P8ai PR #110 validated a real Core frontend HTTP response through the native
Rust host's local HAOS guest, 16/16 CI, merged e891401d65d6f954afb809bd7ff9d7312c7b3e7f.

P8aj adds a deliberately opt-in laboratory systemd unit in
packaging/systemd/rusty-home-haos-lab.service. It has NO Install section and
cannot be enabled on boot. A dedicated unprivileged rusty-home-lab user,
owner-only QCOW2, KVM-only explicit device access, read-only host filesystem,
limited memory/tasks, cgroup cleanup and bounded restart policy are enforced.

New genuinely isolated official HAOS 18.3 GitHub KVM workflow installs unit
under /run/systemd/system on its disposable Ubuntu runner only, starts/stops
the genuine native Rust/QEMU HAOS guest TWICE via systemctl and verifies
no orphan guest, retained original QCOW2 overlay inode, and original official
HAOS backing SHA256. Until exact-feature-SHA CI confirms this, it is code
and an unverified systemd packaging hypothesis, not product certification.
No Cloud9 or household host install/reboot, no radio handover, no 72h soak,
no signed host upgrade path. Release 0/15 fully certified.

## P8ak — installable native Rust HAOS lab Debian package (CI pending)

The opt-in laboratory systemd unit was actually certified in isolated
official HAOS 18.3 KVM by P8aj PR #111: 10/10 checks on feature e1a2c97
and GitHub run 40254861417, merged as
4a1377341eabd44bc81334705465a5ebf33cf6c4.
Two systemd start/stop cycles completed without orphan QEMU and with the
same private COW guest disk and immutable pinned HAOS source.

P8ak turns the runtime proof into a versioned .deb, with its native Rust
binary at /usr/libexec/rusty-home, native unit at /usr/lib/systemd/system,
and a systemd-sysusers account declaration. The opt-in service has NO
Install section and package maintainer scripts create only the dedicated
account, never start/enable/restart home automation. Built reproducibly with
SOURCE_DATE_EPOCH. New actual Ubuntu 24.04 CI installs v0.1.0-1, upgrades
to v0.1.1-1, downgrades to v0.1.0-1, purges package, and verifies no
autostart and no changed real household HAOS.

This LAB PACKAGE IS NOT A SIGNED GENERAL RELEASE. A signed distribution
channel, system reboot/cold boot, full Apps lifecycle and audited production
permissions are future gates. Rusty Home production remains 0/15.

## P8al — signed APT index includes actual native Rust HAOS manager (CI pending)

P8ak PR #112 certified an actual byte-reproducible versioned Debian guest
manager package and real install, upgrade, downgrade and purge on disposable
Ubuntu 24.04; native guest service did NOT auto-start. 11/11 checks including
actual official HAOS KVM (runs 40255870443 and 40255870364), merged
cae2cde2ce1e08eb86bd5a2b0465212be07845cf.

P8al extends the existing P12b authenticated ephemeral signed APT fixture
without introducing any parallel package manager. The same disposable
GPG-signed Release/InRelease and Packages index now includes BOTH versions
of rusty-home-guest-lab alongside the historic read-only bridge. In an
offline Ubuntu client, apt-cache must discover both native versions from
the index only after apt-get verifies its temporary signed-by key. A
tampered InRelease must continue to be explicitly rejected for signature
failure. The GitHub workflow must pass at exact P8al SHA.

This CI key is intentionally discarded and is NOT a production trust
anchor. Native guest .deb installation and rollback were separately proven
under P8ak; this offline client only proves signed index/discovery and
tamper rejection, not a second install of QEMU dependencies. A production
release signing key, safe distribution, Core/Apps operational readiness and
72-hour endurance remain unverified. Product production gates 0/15.

## Immutable verified P8ag–P8al Rust host evidence (26 Oct 2026)

- P8ag PR #108: 14/14 exact feature-head GitHub CI, run 40250349459; long-lived Rust/QMP lifecycle.
- P8ah PR #109: 15/15 exact feature-head GitHub CI, run 40251693506; fixed 127.0.0.1 UI forwarding.
- P8ai PR #110: 16/16 exact feature-head GitHub CI, run 40252930987; real official Core HTTP frontend.
- P8aj PR #111: 10/10 exact feature-head GitHub CI, run 40254861417; real systemd two HAOS/QEMU process lifecycles.
- P8ak PR #112: 11/11 exact feature-head GitHub CI, run 40255870443; reproducible Debian guest package real dpkg lifecycle.
- P8al PR #113: 9/9 exact feature-head GitHub CI, run 40257162323; offline signed APT index and tamper refusal.

All six are MERGED, latest main a756696b60081d641ad7ec9cf43144f3361a201b. Do NOT rerun as newly unproved work. Machine-readable source config/haos-rust-guest-evidence.json keeps each exact original feature SHA, merge SHA and workflow marker source. P8ag–P8aj official HAOS 18.3 QEMU/KVM is an isolated genuine guest; P8ak real Ubuntu dpkg and P8al cryptographic offline APT use disposable CI runners. No household deployment, no signed production trust key, no actual App service parity, no physical radios, no whole-host backwards cutover or endurance qualification. Production remains **0/15** fully certified.

## P8am — native systemd READY and watchdog driven by QMP (CI pending)

Earlier genuine systemd lab P8aj proved two real start/stop cycles, but
Type=exec reported active before QEMU control was ready. P8am adds standard
native Rust sd-notify 0.5.0, Type=notify, NotifyAccess=main and
WatchdogSec=30s. Rust emits READY only after it receives an actual JSON
QMP greeting over the private local UNIX socket, then emits WATCHDOG only
if QMP continues responding. STOPPING is sent on graceful shutdown.
Notification errors use the common QEMU reap path, without guest orphan.

Actual disposable official HAOS 18.3 KVM/systemd CI MUST prove systemctl
start returns after QMP becomes responsive, watchdog timestamps move
during both genuine Rust/QEMU service lifetimes and no QEMU persists after
stop. This does not prove HAOS Core or App readiness, 72h endurance,
real device ownership, or secure unattended upgrades. No household
system modified; production 0/15.
