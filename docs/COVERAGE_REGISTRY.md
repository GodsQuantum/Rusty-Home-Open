# Machine-Checkable Coverage Registry: 9 Apps & 29 Core/HACS Integrations

## Overview

In accordance with CLO-217, CLO-225, and CLO-226, this document defines the machine-checkable coverage manifest and verification contract for representative Home Assistant Core / HACS integrations and Supervisor Apps.

The registry is maintained in `config/coverage-registry.json` conforming to `config/coverage-registry-schema.json`, and validated fail-closed by `scripts/check_coverage_registry.py` and `scripts/test_coverage_registry.py`.

## Anti-Pattern Refusal & Boundary Enforcement

- **Strict NOT_PRODUCTION_READY**: 0 of 15 production release gates are certified.
- **No synthetic PASSES**: No simulated 9/29 passes or made-up completion metrics. Every status of `lab_partial` references an exact GitHub Actions execution receipt (`run_id`, `job_id`, immutable git `head_sha`, and distinct verification scenario `marker`).
- **No 'proven' promotions**: The status `proven` is strictly rejected by the validator until independent multi-host production certification is completed.
- **No VM100 or household access**: All tests originate strictly from disposable, isolated containers or temporary QEMU virtual machines in CI.

## Registered Matrix (2 Apps, 8 Integrations)

### Apps (2 of 9 target)

| Slug | Name | Status | Category | External Receipt | Environment & Scope |
| --- | --- | --- | --- | --- | --- |
| `core_configurator` | File Editor | `lab_partial` | Official Core App | [Run 40574998498](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40574998498) | Disposable HAOS 18.3 Core+Supervisor+File Editor QEMU runner (`P8DN_FILE_EDITOR_HTTP_SAVE_AND_DENIED_INGRESS_PASS`) |
| `core_mosquitto` | Mosquitto broker | `lab_partial` | Official Core App | [Run 40597441250](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40597441250) | Disposable HAOS 18.3 QEMU guest + native Rust atomic escrow (`P8DU_TWO_GENUINE_OFFICIAL_APP_BACKUPS_SAME_GUEST_SHA_VERIFIED_PASS`) |

### Core / HACS Integrations (8 of 29 target)

| Domain | Name | Status | Category | External Receipt | Environment & Scope |
| --- | --- | --- | --- | --- | --- |
| `watchman` | Watchman | `lab_partial` | HACS Integration | [Run 40004128648](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128648) | Pinned Watchman 0.8.4 inside isolated official Core 2026.10.4 Docker container (`P6D_REAL_WATCHMAN_CORE_PASS`) |
| `blueprint_studio` | Blueprint Studio | `lab_partial` | HACS Integration | [Run 40004128648](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128648) | Blueprint Studio 2.5.2 real Core admin vs user API policy in isolated disposable runner (`P6K_REAL_BLUEPRINT_CORE_PASS`) |
| `cup` | CUP Component | `lab_partial` | HACS Integration | [Run 40004128648](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128648) | CUP Component 1.5.0 genuine released asset in official isolated Core (`P6L_REAL_CUP_CORE_PASS`) |
| `card_builder` | Card Builder | `lab_partial` | HACS Integration | [Run 40004128595](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128595) | Pinned Card Builder 2.6.0 release asset, Core config flow and panel asset registration (`P6G_REAL_HACS_CARD_BUILDER_RELEASE_CORE_LIFECYCLE_PASS`) |
| `smart_announcements` | Smart Announcements | `lab_partial` | HACS Integration | [Run 40004128595](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128595) | Pinned Smart Announcements 0.8.3 Core config flow setup, unload, reload, remove (`P6F_REAL_SMART_ANNOUNCEMENTS_PINNED_CORE_PASS`) |
| `adaptive_lighting` | Adaptive Lighting | `lab_partial` | HACS Integration | [Run 40004128595](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128595) | Pinned Adaptive Lighting 1.32.0 config entries, switch entities, and state restoration (`P6F_REAL_ADAPTIVE_LIGHTING_V1_32_0_CORE_LIFECYCLE_PASS`) |
| `custom_universal_media_player` | Custom Universal Media Player | `lab_partial` | HACS Integration | [Run 40004128595](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128595) | Pinned release ZIP v1.9.0 multi-step flow and composite entity in genuine Core (`P6H_REAL_PUBLIC_UNIVERSAL_MEDIA_PLAYER_1_9_0_LIFECYCLE_PASS`) |
| `epson_snmp` | Epson SNMP | `lab_partial` | HACS Integration | [Run 40004128595](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128595) | Pinned Epson SNMP 0.1.3 negative fail-closed absent hardware rejection (`P6J_GENUINE_EPSON_CONFIG_ENTRY_OFFLINE_DENIAL_PASS`) |

## Running Validation

Run the offline manifest validation:
```bash
python3 scripts/check_coverage_registry.py
```

Run unit and adversarial tests:
```bash
python3 scripts/test_coverage_registry.py
```
