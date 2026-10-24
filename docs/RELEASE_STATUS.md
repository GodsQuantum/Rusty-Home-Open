# Public alpha status

No production release or household cutover. Verified synthetic Rust
labs include local HTTP bounded workers (P19) and operating-system-level
exclusive cooperating-process radio fixture lease (P20). True hardware
ownership between hosts or VMs remains wholly unverified.

Home Assistant compatibility experiments are incomplete. All 15
mandatory production release gates remain blocked.

## Verified public certification scorecard

See [the gate-by-gate independent certification ledger](CERTIFICATION_LEDGER.md). NOT_PRODUCTION_READY: 0 / 15 production gates certified.

## Previous real restorations: do not repeat uncritically

The [canonical evidence index](CERTIFICATION_EVIDENCE_REGISTER.md) and [development journal](WORK_LOG.md) document the already-successful 17/17 genuine HAOS QEMU tests, including full, encrypted and File Editor data restoration and a two-version rollback **inside the same disposable VM**. Restoring onto an independent fresh second VM is a distinct, not-yet-proven scenario. These genuine lab successes do not change **0/15 production gates**.

## Newly demonstrated: independent fresh HAOS restore in disposable KVM

[6/6 separate source+target genuine QEMU checks](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40135221183) prove HAOS-to-HAOS full backup migration to a clean second HAOS guest, including exact synthetic share bytes and Core version. This does NOT demonstrate restoring an HAOS backup inside the Rusty Home native runtime. Production remains NOT_PRODUCTION_READY, 0/15 gates. See [canonical evidence registry](CERTIFICATION_EVIDENCE_REGISTER.md).

## Certified evidence vs remaining product development

See [the exhaustive 15-gate production audit](PRODUCTION_CERTIFICATION_MASTERPLAN_2026-10-24.md), [current handoff](HANDOFF_CURRENT.md), and [canonical evidence register](CERTIFICATION_EVIDENCE_REGISTER.md). Based on 76 merged PRs (8/50/18 by UTC date Oct22–24), 9 gates have partial lab evidence and 6 remain blocked; no gate fully certified. The native Rust kernel remains synthetic/experimental and the Core reader lab-only; official HAOS-to-HAOS restoration does not establish replacement of Home Assistant Core.
