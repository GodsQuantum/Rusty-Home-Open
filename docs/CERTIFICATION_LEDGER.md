## 27 octobre 2026 — Deux véritables Apps HAOS simultanées vers un lot atomique Rust [LAB ONLY]

PR #241 fusionnée (main `cd0432ea13de3d6a1a1262db14909af475b6579f`), HEAD source exact `51456cc8d4201b57bba7421005a82dea3a8e36ce`, **10/10 CI verts**. [Run 40597441250](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40597441250), job 122516828609 SUCCESS : **3/3 JUnit** HAOS18.3 officiel QEMU ; **16/16** tests Rust, Clippy et formatage réussis. File Editor et Mosquitto réellement installés ENSEMBLE dans la même VM jetable ; deux sauvegardes Supervisor App-only officielles, SHA-256 et boot identiques vérifiés. Import Rust privé : seconde archive altérée refusée sans publication ; vrai SIGKILL après première App sans publication partielle ; reprise atomique, deux marqueurs fictifs préservés, archives source intactes. Marqueurs CI : `P8DU_TWO_GENUINE_OFFICIAL_APP_BACKUPS_SAME_GUEST_SHA_VERIFIED_PASS`, `P8DU_GENUINE_SECOND_APP_CORRUPT_ESCROW_FAILS_CLOSED_NO_PAIR_PASS`, `P8DU_REAL_OFFICIAL_FIRST_APP_SIGKILL_ZERO_ATOMIC_PUBLICATION_PASS`, `P8DU_TWO_REAL_HAOS_APP_BYTES_ATOMIC_PRIVATE_RETRY_SOURCE_UNCHANGED_PASS`. Identifiant machine unique `haos_real_dual_apps_atomic_sigkill` (le préfixe P8du était déjà employé dans un ancien titre MQTT, ne pas confondre les preuves). **60 preuves laboratoire / 56 runs GitHub distincts ; production 0/15**. Ce test ne démarre ni ne migre deux Apps fonctionnelles dans Rust, ne vérifie ni Ingress authentifié ni matériel, endurance ou audit humain. VM100 et production inchangés.

## 27 October - P8dn actual official File Editor HTTP write and denied ingress [VERIFIED LAB ONLY]

PR #229 MERGED squash main 009c5f1743c827fa93df8b6a8624af7b6a6829ea, exact source HEAD 198966e6a36a6d7fc989160c9cdff05a4b160a9f, 10/10 exact-head checks SUCCESS. Real disposable official HAOS18.3 Core+Supervisor+File Editor QEMU CI 40574998498 job 122438928805 SUCCESS, three of three official QEMU JUnit tests. Trusted internal File Editor HTTP POST /api/save wrote a synthetic guest Core-config file with verified SHA-256; official Supervisor App stop/start retained those exact bytes. Unauthenticated Core Ingress POST was rejected/redirected without creating the file; absent, forged and invalid-cookie GET requests were refused. The real positive POST was from the trusted Supervisor namespace, NOT an authenticated browser session and NOT a Rust-originated write. Machine claim haos_real_file_editor_http_post_write_restart_denial: 55 lab-only claims across 51 distinct successful GitHub runs. App authenticated browser milestone remains blocked; 0/15 production gates certified. No VM100, radios, household data, production service or real mount touched.

# Rusty Home — independent public certification ledger

**Anti-redondance :** consulter le [registre des tests réellement exécutés](CERTIFICATION_EVIDENCE_REGISTER.md) et le [journal du projet](WORK_LOG.md) avant de planifier d'autres essais. Les restaurations complètes, chiffrées et File Editor ont **déjà réussi** dans la même VM HAOS QEMU le 24 octobre 2026 (17/17) ; ceci ne certifie pas une restauration dans une seconde VM vierge.

**Snapshot: 23 October 2026. Status: NOT_PRODUCTION_READY.**

**0 / 15 mandatory production release gates certified.** Nine gates now have
partial laboratory evidence; none is an independently qualified production gate.
The nine sample Apps published here are all fictional synthetic fixtures.

## 27 October — P8cu genuine HAOS App-only partial archive

PR #206 MERGED squash `e46306a80525ab557f73e181f6113930df186b2b`, exact source `5839675720581f07c9e90f2c4052bf0c6b90d99e`; **10/10 GitHub checks SUCCESS**. Genuine official HAOS18.3 disposable QEMU File Editor install and Supervisor App-only partial backup, SHA-256 private relay, exact nonsecret marker recovered from real nested App payload with read-only inspection: [CI 40514471305](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40514471305), job 122224674821, JUnit **3/3**. Claim `haos_real_selected_app_only_partial_backup_readonly_preflight`; **44 lab claims / 40 distinct successful runs**. This does not restore an App to Rust or authorize migration, production **0/15**.

## 27 October — P8cv exact-head genuine Redis digest-switch crash proof

PR #207 MERGED squash fec9636f1f5f40e4764be47273586f6abffeb3f2; exact source 0cfc4b4a32a218048043115a9011108433ffdb3c, **30/30 SUCCESS**. Disposable real Redis 7.4 image-switch coordinator SIGKILL, private fsynced journal, all mutators fenced, symlink journal denied, exact old digest and RDB restored by independent process, normal switch/rollback regression and persistent volume preserved: [CI 40506098662](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40506098662), job 122195673271. Claim oci_native_image_switch_sigkill_durable_recovery, **43 lab claims / 39 distinct successful run IDs**. This is NOT an automatic reboot recovery, database schema rollback, whole HAOS migration or production certification. **0/15 independent gates certified.**

## 27 October — P8ct heterogeneous native pair exact CI evidence

Merged PR #204 at `162ef61540c384567f2a0277936b3d9dfc85dc5a`, original exact head `0b1338aad59a07390efadaef597636c47c5e541f`: **29/29 checks SUCCESS**, disposable real Redis + Meilisearch crash-recovery run [40498864752](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40498864752), job 122171170744 SUCCESS. Seven distinct P8CT scenario markers, Redis RDB and Meili index durable after coordinator SIGKILL and new-process stop-only recovery; original volumes untouched. P8cn race regression green run 40498864583, not a new claim. Evidence ID `oci_native_heterogeneous_pair_sigkill_durable_data_recovery`, **42 lab claims / 38 successful run IDs**, release 0/15 certified, no independent signoff.

## Product development evidence — not a production gate

- Supervisor job discovery (work in progress, independent PR CI pending):
  native Rust GET /jobs/info V1/V2, admin-only authenticated Ingress
  GET /api/jobs, redacted 24-job UI and TCP/adversarial tests. TDD failed
  with E0432/E0599 before implementation; 5 new Rust integration tests,
  full Rust workspace, strict Clippy, all-features Supervisor tests and
  frontend syntax passed locally in isolated LXC700 DEV. See
  [scoped implementation evidence](SUPERVISOR_JOBS_DISCOVERY.md).
  No operation execution, persistent receipt or production authorization.

## Verified, narrowly scoped laboratory results

| Area | Real verified result | GitHub Actions evidence |
| --- | --- | --- |
| Rust native | Bearer auth, state restart, real SIGKILL recovery, fencing, bounded HTTP and synthetic radio flock | [Native Rust](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39868514395) |
| Home Assistant Core | Real Core 2026.10.1, 1,232 synthetic states via private UDS to Rust, stale HTTP 503 | [Core bridge](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39870909461) |
| Selected HACS | Eight authentic integration lifecycles on real Core, not 29/29 certification | [First three](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39870909461), [five more](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39871512116) |
| HAOS/Supervisor | Real HAOS 18.3 KVM: 17/17 official lifecycle, full and encrypted same-guest restore, File Editor SHA256 data, rollback; separate FRESH source→target HAOS disaster recovery 3/3 + 3/3 | [Same-guest HAOS](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40127707580), [independent fresh second VM](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40135221183) |
| HAOS→native Rust identity stage (P8aa) | Official HAOS 18.3 Supervisor full unprotected archive, SHA256 transfer, two fictional Core identities in durable Rust/redb, CAS activate + rollback (3/3 genuine QEMU) — **NOT** whole Core/Apps restore | [P8aa run 40238122942](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40238122942) |
| Debian package | Reproducible test .deb install/update/downgrade/purge and cryptographic APT tamper rejection | [Debian and APT](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39872009683) |
| Security fuzz | Real sanitizer-instrumented libFuzzer Core user parser with synthetic fixtures for 45 seconds | [Fuzz CI](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39872710558) |

All of these are valid experiments with specific scopes. None accesses a
real household, proves all 29 custom components or all nine Apps, or
authorizes a hardware radio handoff or real migration.

## Product development evidence (NOT production certification)

- **2026-10-23, independent read-only Core session verification (new PR,
  independent CI pending):** Rust backend now binds an accepted official
  Supervisor Ingress TCP identity to a fresh authenticated Core WebSocket
  `config/auth/list` response through a fixed internal Supervisor endpoint.
  `GET /api/session/core` reports only active human admin/member status and
  always returns `actions_authorized=false`. Unknown, disabled, system
  accounts and Core outages fail closed. The dashboard distinguishes
  Supervisor group claims from Core-confirmed status. TDD initially failed
  with E0425/E0433, then new adversarial HTTP/role/revocation/transport
  tests passed; Rust workspace all-features, strict Clippy, rustfmt, JS
  syntax and public-fixture checks passed locally in isolated DEV.
  Browser/HAOS independent evidence is **pending**; real Core permissions
  may deny user enumeration. No write permit, production auth certification
  or release gate is inferred. [Scope](CORE_VERIFIED_INGRESS_SESSION.md).


- **2026-10-23, read-only Supervisor Job progress ([PR #45](https://github.com/GodsQuantum/Rusty-Home-Open/pull/45), MERGED):**
  Exact independently validated head `d781cec80f234334903cf5739b0e93f2fd97bcd5`,
  squash merge `126fc9c2d26c24160b6be06b1a5be0d66848c748`.
  Eleven independent checks passed, including genuine disposable HAOS 18.3 KVM.
  native Rust route and French dashboard poll official V1/V2 job endpoints
  by strictly validated UUID; output contains only state, progress, ID and
  error count, never raw stage, error messages or child jobs.
  Four control-plane adversarial tests and one authenticated TCP loopback
  integration test pass locally; full workspace/Clippy and independent CI
  passed on the exact merged PR head.
  No user RBAC, trusted operation receipt or actual App mutation is implied.
  [Scope and limits](LIVE_SUPERVISOR_JOB_PROGRESS.md).

- **2026-10-23, official installed-App release notes ([PR #48](https://github.com/GodsQuantum/Rusty-Home-Open/pull/48), independent CI pending):**
  real Rust V1/V2 Supervisor HTTP read of arbitrary App changelogs after
  current installed-inventory validation, bounded text-only transport and
  on-demand native dashboard display. Red-first TDD E0432/E0599 followed
  by three adversarial loopback transport tests and one control-plane
  route test passing. Full Rust workspace, strict Clippy, format and
  read-only smoke passed in isolated DEV. Browser synthetic regression
  added, independent CI pending. **No App writes, production user RBAC,
  HAOS field authorization or release certification.**
  [Scope](LIVE_SUPERVISOR_APP_CHANGELOG.md).


- **2026-10-23, live installed-App telemetry (product feature, PR pending):**
  native Rust V1/V2 Supervisor stats client, current installed-inventory
  gating, numeric-only redacted API and dashboard resource panel for
  arbitrary App slugs. Code commit
  1a5a0fe98bec4936b47438a78ead4163b5fd7098.
  TDD E0432/E0599 failure reproduced; four new adversarial loopback tests,
  Rust workspace all-target tests, strict Clippy, rustfmt, JavaScript syntax
  and public fixture guard passed in isolated LXC700 DEV.
  **Independent CI on the final PR SHA remains pending.**
  This does not certify production user RBAC, write permissions, HAOS
  integration, or any release gate.
  [Functional scope](LIVE_SUPERVISOR_APP_STATS.md).

- **2026-10-23, PR #37:** real read-only Supervisor Store App details for
  installed and uninstalled Apps, arbitrary vendor slugs, native Rust
  authenticated HTTP client, current Store membership validation and
  user-facing dashboard. Code commit:
  `8d1654b11fbcd8d19b099bf96a3b4bc43b7e617e`.
  TDD initially failed for missing methods/routes, then all Rust workspace
  all-target tests, strict Clippy and dashboard JavaScript syntax passed in
  isolated DEV. Independent GitHub CI, including disposable HAOS KVM,
  succeeded on refreshed head `fb25ae2eda428a8b71df94a7a136c74cef8e596a`
  ([HAOS run](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39972631127));
  merged as `10bb01b2e57e81f0c2276c8342125da1baf54d46`.
  These tests do NOT prove App lifecycle write authorization. See
  [Store detail scope](STORE_LIVE_DETAILS.md) and
  [PR #37](https://github.com/GodsQuantum/Rusty-Home-Open/pull/37).

- **2026-10-23, PR #36 merged:** actual Rusty Home product App installed,
  started, checked non-root and cleaned up in disposable official HAOS 18.3
  KVM. [Independent HAOS workflow](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39966193068)
  succeeded on PR head `82c51013c8ab4942138d621fff6d4d304e6deeff`;
  merge commit `a36cd02a35361af8a272b7b81ea5614990df97dc`.
  This does not establish human UI authentication or migration readiness.
- **2026-10-23, Store availability preflight:** Rust product code
  `afc1b6065771db60367d65357e7f83154f09b416` and installed-inventory
  reconciliation `260fac994745a1bc7e5a1fc5288ace689c3a4506` add live V1/V2
  availability reconciliation, fail-closed typed read-only API and dashboard.
  TDD first failed at E0432, then seven adversarial native tests, two
  control-plane tests, full Rust workspace all-target tests, strict Clippy,
  rustfmt and dashboard JavaScript syntax passed in isolated LXC700 DEV.
  Independent CI on final PR SHA is **pending**, not certified.
  [Preflight scope](STORE_AVAILABILITY_PREFLIGHT.md).

## Scoped laboratory checks with pinned GitHub evidence

**15 independently observed successful GitHub Actions jobs** are recorded in
[the machine-readable evidence index](../config/lab-evidence.json),
including exact run URLs and commit SHAs. This certifies only the stated
**isolated laboratory test results**, never production operation, user data
or full Home Assistant parity.

| Lab check | Production gate informed (not cleared) | Exact CI evidence |
| --- | --- | --- |
| `debian_real_dpkg` | `packaging_updates` | [run 40004128778](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128778) |
| `apt_signed_tamper_refusal` | `packaging_updates` | [run 40004128778](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128778) |
| `official_core_rust_bridge` | `ha_core_hacs_29_runtime` | [run 40004128648](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128648) |
| `selected_hacs_three` | `ha_core_hacs_29_runtime` | [run 40004128648](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128648) |
| `adaptive_media_hacs` | `ha_core_hacs_29_runtime` | [run 40004128595](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128595) |
| `epson_snmp_hacs` | `ha_core_hacs_29_runtime` | [run 40004128595](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128595) |
| `card_announcements_hacs` | `ha_core_hacs_29_runtime` | [run 40004128595](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128595) |
| `native_rust_security_suite` | `authenticated_host_transport` | [run 40004128682](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128682) |
| `official_product_nonroot` | `authenticated_host_transport` | [run 40004128675](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128675) |
| `official_marketplace_abi` | `ha_core_hacs_29_runtime` | [run 40004128906](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40004128906) |
| `official_haos_kvm` | `supervisor_apps_restore` | [run 40002933713](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40002933713) |
| `browser_chromium` | `production_ui_auth` | [run 40002934403](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40002934403) |
| `security_fuzzer` | `security_stress_fuzz` | [run 39997355066](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39997355066) |
| `fuzz_four_native_security_surfaces` | `security_stress_fuzz` | [run 40026397117](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40026397117) |
| `github_oidc_cargo_source_sbom` | `third_party_licenses` | [run 40027460088](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40027460088) |

The evidence index passes local schema and tampering tests. Each run and
successful named job was independently checked against GitHub on 23 October 2026;
an offline index is not a cryptographic certification or a substitute for
rechecking current external provenance. Independent production signoff
and the 15 mandatory release gates are separate.

## Release gate matrix — every gate remains noncertified

| Exact gate identifier | Machine state | Production acceptance still needed |
| --- | --- | --- |
| `authenticated_host_transport` | `lab_partial` | Least-privileged authenticated live transport, expiry, outage recovery and adversarial audit |
| `ha_core_hacs_29_runtime` | `lab_partial` | Complete lifecycle coverage for all 29 targeted custom integrations |
| `hacs_github_oauth` | `lab_partial` | Genuine OAuth login, renewal, revocation, recovery and stale-session tests |
| `apps_9_lifecycle` | `blocked` | Install, start, update, stop and rollback for each of nine actual Apps |
| `supervisor_apps_restore` | `lab_partial` | Full App persistent data, ingress and service restore under Supervisor |
| `haos_backup_restore` | `lab_partial` | Protected full archive/data recovery end-to-end with user-equivalent data |
| `rollback_verified` | `blocked` | Revert actual migration including states, persistent data and fabric pairing |
| `single_radio_owner` | `blocked` | Exactly one real Zigbee, Thread, Matter and other device owner at each transition |
| `ha_ws_reconnection` | `lab_partial` | Real Home Assistant WebSocket reconnect and long-duration outage/loss tests |
| `production_ui_auth` | `blocked` | Full session, identity, role, token revocation and API authorization testing |
| `native_drivers_services` | `blocked` | Prove real device driver, service and system integration parity |
| `performance_full_stack` | `blocked` | Representative whole-stack scale, fault tolerance, resources and soak |
| `packaging_updates` | `lab_partial` | Production-signed distributions, installed real upgrades and rollback |
| `third_party_licenses` | `lab_partial` | Independent binary-release BOM, attribution, asset/firmware notices and redistribution review |
| `security_stress_fuzz` | `lab_partial` | Full attack-surface coverage, independent penetration test and prolonged security soak |

## Effort estimates — not a certification score

- **About 80% of a laboratory alpha:** a subjective engineering estimate.
- **About 30% of a full Home Assistant replacement:** a subjective
  engineering-effort estimate with substantial uncertainty.
- **Precisely 0 / 15 production gates certified:** the objective result.

Hardware ownership, all selected HACS integrations, App data, OAuth and
real reversible migrations could reveal substantial unplanned work.
No date of production completion can be established from isolated labs.

## Recommended independent certification order

1. Review third-party license obligations and published assets.
2. Finish authenticated transport, browser access and 29 HACS lifecycles.
3. Prove nine App data and lifecycle migrations, then backup and rollback.
4. Prove real single-radio ownership under faults, full-stack load and soak.
5. Record independent decisions and artifacts for each of the 15 gates.

The only machine authority for production release gates is
[config/release-gates.json](../config/release-gates.json).


## Product development evidence: dynamic Supervisor Store repositories (2026-10-23)

- Feature commit 371c0a65d1005274b47c9704084791b0bb615c3c, PR #51.
  Independent GitHub Actions: nine checks passed on head
  af198eb236adf0202ada25d594913dd9a5ec6d48; PR #51 merged as
  32804326847cd8f9cf375aa42a2bd7060aac49b2.
- Real UI increment on a feature branch: a repository dropdown derived from
  the official Supervisor Store response, exact vendor-neutral filtering,
  combined search/paging and fail-closed invalid metadata handling.
- TDD: Python UI contract initially failed on the missing selector, then
  passed. Node executes the actual browser filter/validator against 1317
  fictional Apps, 23 repositories and malicious/duplicate metadata.
- Independent Playwright browser scenario is included in
  scripts/capture_branding_lab.mjs; **not yet independently certified**
  on the exact PR SHA (successful screenshot workflow).
- No App lifecycle mutation, production identity or gate signoff. All 15
  production gates remain blocked/lab_partial.


## Product security increment: restrict direct Supervisor job detail (PR #56)

- Direct `GET /api/jobs/{uuid}` now has the same **current** verified
  Supervisor Ingress administrator requirement as job discovery; a known UUID
  cannot be used from loopback or by a non-admin. Browser job selection and
  polling are revoked on session refresh, loss or role downgrade.
- TDD RED: Rust E0061 (new five-argument access test against old unguarded
  handler); GREEN: dedicated adversarial test, workspace tests, strict Clippy,
  rustfmt, read-only HTTP smoke, JS syntax and synthetic repository tests
  passed in isolated LXC700 DEV.
- Independent CI on the exact new PR head remains to be checked before merge.
  This does **not** verify independent Core RBAC or enable App writes. All
  production gates remain blocked/lab_partial.

## Product correction: fail-closed Supervisor job reconciliation (2026-10-23)

- A completed Supervisor job and an App state snapshot are **not** sufficient
  evidence if either is marked non-read-only, the job progress is malformed,
  or the installed-App inventory ambiguously duplicates a slug.
- Native Rust reconciliation now refuses such forged/inconsistent evidence,
  including a successful job carrying an error count. This applies to arbitrary
  vendor slugs and both Supervisor API versions through the shared reconciler.
- Regression test `untrusted_job_or_inventory_never_confirms_completion`
  reproduced a false `Confirmed` before the correction.
- This is a read-only product correction, **not** durable actor-bound receipts,
  authorization for App mutations, or end-to-end production certification.
  Independent CI evidence belongs to the matching PR and Issue #12.

## Examen individualisé des 15 critères (26 octobre 2026)

La commande canonique `python3 scripts/check_release_gate.py --matrix` affiche
pour chaque gate son statut laboratoire, son verdict **production** indépendant,
les quatre scénarios obligatoires et ses bloqueurs précis déjà présents dans
`config/release-gates.json`. Aucune CI verte ou test de laboratoire n'autorise
le passage en production sans preuve indépendante vérifiée, double signature
Ed25519 puis approbation explicite de publication. Statut actuel : 9 partiels,
6 bloqués, **0/15 certifié** ; aucune restauration HAOS précédemment prouvée
n'a été répétée.

## Future independently signed criterion promotion

The `check_release_gate.py --matrix` machine guard remains the only authority for **current** accepted per-gate technical evidence. Snapshot statements above describe laboratory status on 26 October 2026. The CI regression suite now allows a future single criterion to change to `certified` ONLY after its precise required scenarios and independent dual Ed25519 signatures have passed; the live manifest and ledger must then be updated together. Historical masterplan and archived HAOS fixture claims are not silently rewritten. Even 15/15 accepted technical gates never authorize production deployment without an additional explicit independent release signoff. This is not a claim that any gate has been certified today.
