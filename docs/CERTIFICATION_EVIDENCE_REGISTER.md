# Rusty Home — registre canonique des preuves et anti-redondance

**Mise à jour : 24 octobre 2026.** Ce fichier est la référence pour éviter de refaire inutilement les essais. La source structurée vérifiée par CI est [`config/certification-evidence-registry.json`](../config/certification-evidence-registry.json). Toute nouvelle validation doit y être inscrite **avec le test, l'environnement, le commit exact, le run GitHub et les limites**, puis ajoutée au [journal du projet](WORK_LOG.md). Ne jamais remplacer une preuve existante par une estimation ni confondre test de laboratoire et gate de production.

## Réponse à la question « la restauration a-t-elle déjà été testée ? »

**Oui, plusieurs fois sur du véritable HAOS 18.3**, en QEMU/KVM isolé. [Campagne prouvée : 17/17 tests GitHub, zéro erreur](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40127707580), commit exact `8dde4a49315ee009c1c45341d5730f17668ea8f3`. Il ne faut **pas** refaire ces scénarios en les présentant comme encore absents.

| Identifiant stable | Scénario déjà couvert | Résultat / portée |
| --- | --- | --- |
| `haos-full-backup` | Archive complète Supervisor, SHA-256 | Vérifié en QEMU |
| `haos-full-restore-same-guest` | Données `share` corrompues puis restauration complète, identité Core | Vérifié **dans la même VM** |
| `haos-encrypted-full-backup-restore` | Backup complet protégé, données altérées puis restaurées | Vérifié **dans la même VM** |
| `haos-wrong-password` | Mot de passe erroné : refus sans mutation des données | Vérifié en QEMU |
| `haos-real-file-editor-partial-data` | File Editor réel : restauration du contenu avec SHA-256 | Vérifié en QEMU |
| `haos-versioned-app-rollback` | App locale V1→V2, mutation, manager SIGKILL, retour V1 et SHA-256 | Vérifié en QEMU |
| `haos-sync-install-release` | Installation native synchrone, inventaire officiel puis déverrouillage ACID | Vérifié en QEMU |
| `haos-async-expired-job-fence` | Job terminé et effacé : aucune reprise dangereuse de POST | Vérifié, **fail-closed seulement** |

Les scripts sources sont `scripts/p8c2_real_haos_supervisor_pytest.py`, leur sélection CI `.github/workflows/p8c2-haos-qemu.yml` et le contrôle strict `scripts/check_p8d_junit.py`. **Le compteur 17/17 désigne tous les scénarios sélectionnés, pas huit tests de restauration ni 17 intégrations certifiées.**

L'ancienne campagne [du 22 octobre 2026](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39873207970) avait déjà validé 16/16 ; la campagne plus récente 17/17 est la référence.

## Nouvelle preuve : restauration sur DEUX VM HAOS différentes

**Réussie le 24 octobre 2026.** La campagne [GitHub 40135221183](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40135221183), SHA exact 8d3b1263f0cb65f11fdbb9f3979a58af052b3731, a réussi **3/3 tests source + 3/3 tests cible**, sans saut. Elle est séparée des 17/17 tests SAME-VM ci-dessus.

La VM source produit une vraie archive complète Supervisor, l'envoie à un relais local borné et enregistre son SHA-256 et boot ID. La deuxième VM part du disque officiel HAOS 18.3 intact, doit avoir un boot ID différent et aucun fichier marqueur source ; elle télécharge le tar, vérifie la taille et le SHA-256, importe puis restaure la sauvegarde avec la vraie CLI HAOS, et démontre la récupération des octets exacts et de Home Assistant Core/Supervisor. Voir [protocole et limitations](CROSS_VM_DISASTER_RECOVERY_2026-10-24.md).

**Ce qui reste NON prouvé :** la migration d'un foyer HAOS vers le moteur **Rusty Home**, les bases de toutes les applications tierces, le remplacement de Core et du Supervisor, les contrôleurs physiques, comptes réels, réseau et rollback d'un matériel entier. La seconde VM prouve HAOS→HAOS, pas HAOS→Rusty Home.

## Preuves nouvelles — Core officiel vers Rust/Moli (#89–#95)

Ces sept résultats indépendants sont désormais consignés dans le [registre de migration native structuré](../config/native-migration-evidence.json). Les run GitHub cités ont été vérifiés en `success` au **SHA source exact** le 25 octobre 2026, et les sept PR sont fusionnées. Les scénarios sont différents de la restauration HAOS→HAOS documentée ci-dessus.

| Identifiant stable | Preuve et portée maximale | Source vérifiée |
| --- | --- | --- |
| `core-registry-stage` | #89 : véritable Core 2026.10.2, identités vers redb Rust et rollback, sans états ni action | [GitHub 40140787860](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40140787860) |
| `core-registry-real-moli-hub` | #90 : vrais identifiants Core dans le véritable Hub Moli, lecture seule | [GitHub 40141864751](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40141864751) |
| `core-real-state-snapshot` | #91 : états synthétiques générés par le vrai Core vers Moli, instantané borné | [GitHub 40142758906](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40142758906) |
| `loopback-event-convergence` | #92 : événements WebSocket fictifs authentifiés vers Moli, déconnexion et révocation | [GitHub 40144794037](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40144794037) |
| `bounded-ws-moli-runner` | #93 : binaire Rust invocable, WebSocket loopback fictif, sortie bornée | [GitHub 40145513687](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40145513687) |
| `official-core-ws-to-moli` | #94 : authentification et vrais événements WebSocket Core officiel→Moli sur réseau Docker inexistant, avec rollback | [GitHub 40211747111](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40211747111) |
| `official-core-ws-moli-consistency` | #95 : huit événements Core, refus de mauvais token, cohérence des états et révocation fail-closed | [GitHub 40213008548](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40213008548) |

La présence d'un chemin Core→WebSocket→Rust→Moli **ne valide PAS** le remplacement de Home Assistant Core, les services ou automatisations, les contrôleurs radios, la migration des données de foyer, les intégrations tierces au complet, une reconnexion durable ou une restauration HAOS→Rust→HAOS. Ce sont des preuves de laboratoire bornées avec de vraies images Core ; le compteur reste **NOT_PRODUCTION_READY — 0/15**.

## P8aa — véritable archive Supervisor HAOS vers identités Rust certifiée en laboratoire

Le nouveau Rust lit un tar full Supervisor non chiffré et importe uniquement core.entity_registry vers redb, avec rollback. CI prévue : vrai Core 2026.10.2 dans un tar synthétique et, séparément, vrai Supervisor HAOS 18.3 en VM jetable produisant un tar avec identités fictives, importées en Rust. La source, le test et leurs limites sont consignés dans docs/HAOS_TO_NATIVE_BACKUP_STAGE_2026-10-25.md. Résultat VÉRIFIÉ : run https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40238122942, SHA PR #99 9c6e3220f433153c0dc28ef0d2549bd52a345a8f, merge d297bbbd29ac313723b54794ad27aceaf0636166 ; 3/3 tests sélectionnés dans HAOS 18.3 QEMU, archive officielle full non chiffrée, import du registre (2 identités fictives) dans redb, activation CAS et rollback. Preuve durable dans config/haos-native-backup-evidence.json. Ceci ne prouve ni restauration complète Core, ni Apps, ni automatisations, ni secrets. Les sept preuves Core→Rust historiques et HAOS→HAOS restent acquises, production 0/15.

## Protocole obligatoire pour toute nouvelle session

1. Lire **ce registre**, [le journal](WORK_LOG.md), [les gates](CERTIFICATION_LEDGER.md), `scripts/check_p8d_junit.py`, puis contrôler les statuts et commits GitHub actuels.
2. Pour chaque tâche proposée, chercher le cas par identifiant stable, vérifier sa couverture et son *périmètre* ; préférer améliorer la lacune distincte plutôt que réexécuter la même preuve.
3. Utiliser une branche et un worktree isolés dans `/home/arezki/Projets/Applications/` (LXC 700). Aucun essai destructif sur le foyer. Conserver l'état `NOT_PRODUCTION_READY`.
4. Modifier le registre JSON **dans la même PR** que les preuves, mettre à jour ce Markdown et inscrire date, sha, PR, tests et risques dans `WORK_LOG.md`. Le script `scripts/test_certification_evidence_registry.py` refuse une preuve dont le nom n'est plus compilé dans la campagne HAOS.
5. Attendre les CI GitHub **du SHA exact**, fusionner après tous les checks requis verts et écrire l'issue dans le journal. Ne déclarer un gate de production certifié qu'avec la procédure de qualification indépendante dédiée.

**Production : NOT_PRODUCTION_READY, 0/15 gates certifiés.** Un test réussi ne transforme pas automatiquement un gate en certification de production.

## P8ab — Native Rust archive custody and genuine second HAOS guest

New distinct proof (not old direct HAOS->HAOS): PR #101 at 58cbfe3023532b7acca68123641a58692fab541d, merged 4757377d5d773844f6e278af4712597d50cd799a, GitHub https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40240344194 SUCCESS. Both independent genuine HAOS 18.3 QEMU source 3/3 and target 3/3 pass, with opaque byte-exact private native Rust escrow in the middle, file replaced by native Rust reexport and verified before target restore. This does not qualify actual host takeover or real Apps/radios; production 0/15. Machine evidence in config/native-backup-escrow-evidence.json.

## Native Rust guest-host evidence P8ac–P8af — independently green genuine HAOS KVM (26 October)

P8ac PR #103, official HAOS serial boot via Rust process, GitHub 40243098816 (18/18); P8ad PR #104, official Supervisor AND Core CLI health, GitHub 40243728521 (11/11); P8ae PR #105, official HAOS COW persisted across QEMU SIGKILL restart, GitHub 40247000475 (12/12); P8af PR #106, second independent Rust manager reattached with exclusive flock and rejected concurrent writer, GitHub 40247834775 (13/13). All merged into main d8c5bee817c7b7e8ef5ea6af24d62d366fbe610a. Exact approved HEAD SHA and workflow proof markers in config/haos-rust-guest-evidence.json. Real 18.3 QEMU lab, fictional data only; no family access. They prove usable native Rust host primitives, not a distributable host daemon, secure operational App service, radio, hardware handover, full reverse migration or 72-hour reliability. Production 0/15.

## Immutable verified P8ag–P8al Rust host evidence (26 Oct 2026)

- P8ag PR #108: 14/14 exact feature-head GitHub CI, run 40250349459; long-lived Rust/QMP lifecycle.
- P8ah PR #109: 15/15 exact feature-head GitHub CI, run 40251693506; fixed 127.0.0.1 UI forwarding.
- P8ai PR #110: 16/16 exact feature-head GitHub CI, run 40252930987; real official Core HTTP frontend.
- P8aj PR #111: 10/10 exact feature-head GitHub CI, run 40254861417; real systemd two HAOS/QEMU process lifecycles.
- P8ak PR #112: 11/11 exact feature-head GitHub CI, run 40255870443; reproducible Debian guest package real dpkg lifecycle.
- P8al PR #113: 9/9 exact feature-head GitHub CI, run 40257162323; offline signed APT index and tamper refusal.

All six are MERGED, latest main a756696b60081d641ad7ec9cf43144f3361a201b. Do NOT rerun as newly unproved work. Machine-readable source config/haos-rust-guest-evidence.json keeps each exact original feature SHA, merge SHA and workflow marker source. P8ag–P8aj official HAOS 18.3 QEMU/KVM is an isolated genuine guest; P8ak real Ubuntu dpkg and P8al cryptographic offline APT use disposable CI runners. No household deployment, no signed production trust key, no actual App service parity, no physical radios, no whole-host backwards cutover or endurance qualification. Production remains **0/15** fully certified.


## Final merged proof ledger P8am–P8as — 26 October 2026

- P8am PR #115: 26/26 checks green at original feature head, GitHub Actions run 40261132205; real systemd QMP READY and watchdog on official HAOS KVM.
- P8an PR #116: 11/11 checks green at original feature head, GitHub Actions run 40263269849; real QEMU SIGKILL and automatic systemd service recovery on same HAOS disk.
- P8ao PR #117: 20/20 checks green at original feature head, GitHub Actions run 40264974609; native Podman Quadlet generator parses two generic digest-pinned OCI Apps.
- P8ap PR #118: 21/21 checks green at original feature head, GitHub Actions run 40266019276; real rootless Redis OCI App two lifecycle rounds and persistent data.
- P8aq PR #120: 11/11 checks green at original feature head, GitHub Actions run 40290122155; real Redis volume backup SHA and tampered archive refusal, separate restore.
- P8ar PR #121: 23/23 checks green at original feature head, GitHub Actions run 40291069681; Rust native manager installs, starts, stops and removes App but retains data.
- P8as PR #122: 24/24 checks green at original feature head, GitHub Actions run 40292676656; explicit 127.0.0.1 rootless Podman TCP with authenticated Redis PING.

These seven PRs are MERGED, latest base main 95123e09ac73d63da28e6106fb63794cc2fa85af.
There are now TWO separate machine-readable evidence ledgers:
config/haos-rust-guest-evidence.json (12 host/packaging cases (10 genuine HAOS/KVM and 2 real Ubuntu DEB/APT cases)
P8ac–P8an, excluding p8ao onward) and config/native-oci-app-evidence.json
(5 genuinely running rootless Podman/Redis cases P8ao–P8as). Original PR
head SHA, GitHub workflow run, merged commit, distinct real marker,
and exact CI count are pinned and checked on every PR. Do NOT redo
previous passed stages merely because a later stage is incomplete.

P8as 127.0.0.1-only host INGRESS does not prohibit container outbound
egress. A real Redis test does not imply general Home Assistant Supervisor
App/HACS parity, real physical radios, operator-authenticated production
writes, security/availability endurance, secure signed distribution or a
complete HAOS→Rust→HAOS household cutover. Product 0/15 fully certified;
no family HAOS system modified or rebooted.

## 26 October 2026 — P8at–P8az immutable independent OCI proof reconciliation

These six distinct cases were already completed in real, disposable GitHub CI and merged. Their exact SHA/run/marker is now pinned in `config/native-oci-app-evidence.json`; **do not rerun** unless a material version/code/environment change invalidates a specific test. All cases are lab only, no household or genuine production secret was used.

| Case | PR | Original head | Merge | Run ID | Checks | Actual new scoped behavior |
|---|---:|---|---|---:|---:|---|
| P8AT | #124 | `f11a8022` | `42db2dc1` | 40295014593 | 25/25 | generic oci sha256 image upgrade and explicit rollback |
| P8AU | #125 | `fa64f294` | `05008d4c` | 40297108559 | 26/26 | real redis native oci healthcheck automatic incompatible image rollback |
| P8AV | #126 | `a4f66d11` | `6ed03afa` | 40299977840 | 10/10 | simultaneous distinct rootless redis and memcached generic lifecycle |
| P8AW | #127 | `be76a29a` | `b1881a64` | 40301611933 | 28/28 | stateful docker distribution registry custom oci data volume http |
| P8AY | #130 | `3004210c` | `075e18e5` | 40314671475 | 18/18 | real rootless oci lifecycle lock install start remove reinstall |
| P8AZ | #131 | `ddcd3550` | `a351529c` | 40315761886 | 20/20 | native podman secret reference real rootless redis sha256 mount |

The existing five P8ao–P8as immutable proofs remain intact. The test suite verifies all eleven cases and their exact original sources; production still has 0/15 accepted independent gates.

## Official Home Assistant Core 2026.10.4 independent additional baseline (26 October 2026)

PR [#134](https://github.com/GodsQuantum/Rusty-Home-Open/pull/134) verified genuine official **Core 2026.10.4** (released 24 October) inside a disposable GitHub Ubuntu runner with no network route, nonroot, readonly Docker, independently from preserved Core 2026.10.2 and 2026.11-dev Marketplace baselines. The original independently verified feature HEAD is `21d9b21421a215856930764f2264509f06459a3a`, passed 10/10 checks, GitHub run [40319204128](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40319204128), merged `16a1f49b9b17e2175e30268cd4fc545b2121a05f`. Synthetic Core config entry lifecycle covers real create/load/unload/reload/remove, strict version, no device IO. The initial CI run 40318906283 FAILED due erroneous test import `homeassistant.__version__` (correct official import `homeassistant.const.__version__`), resolved and independently rerun. This is a new version check, not an extra HACS full parity test and NOT any production gate; **0/15 independent production gates**.

## P8be + P8bf — exact original automations source custody and reverse HAOS restore, real KVM (26 October)

**New independent scoped evidence, not a production certificate:** PR #136 original head 3527be53ac91e3a329e3289772786d4a73a1d9f9 merged as 004f2cb52c9993cff34653f133e8c31919585803 (12/12 PR CI), [genuine HAOS QEMU run 40328174220](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40328174220). The actual Supervisor full tar containing an inert, invented automations.yaml is read by Rust and persisted byte-for-byte in an inactive owner-private redb, paired with real archive SHA256 and imported entity-registry digest, with no service invocation.

PR #137 original head ed07cc772951079347eaaa08234623e2b0674f55 merged as 18b519e9b91451fed38a22665a9109ca7682225b (12/12 PR CI). **Two different genuine HAOS 18.3 boot/restore tests:** [40330463271](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40330463271), official backup restored to second fresh HAOS; and [40330463267](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40330463267), same principle via native Rust byte-exact escrow before official second-guest restore. Both validated the original automation source SHA256 on the fresh target, which lacked this fixture beforehand. No YAML was executed.

Structured anti-duplication evidence is APPENDED to existing config/haos-native-backup-evidence.json under additional_automation_cases, keeping historical P8aa original proofs intact. The real HAOS guest is disposable; VM100, actual radios, family automations and credentials never touched. **Not demonstrated:** HA YAML→Moli executable translation and true HAOS→Rust live takeover→HAOS cutover, integrations, all App DBs, and user permissions. Production still NOT_PRODUCTION_READY, 0/15 gates independently signed.

## P8bh / P15 — new actual Core WS recovery and importer sanitizer receipts (Oct 26)

- **official-core-ws-native-reauth** : PR #139 head 83b90ab141ffe79af09e08915281afc44f9078f6 merged 282875112efa332fccd440534dd45f7d3c53d163, 9/9 green. Genuine official Core 2026.10.2 WebSocket → Moli proof in GitHub run [40340882977](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40340882977). The original native Hub is cleared/revoked, an invalid token denied by official Core, and a new authenticated WS subscription + Core snapshot builds a NEW native Moli Hub. Scope is a local disposable simulated failure, not a 72h recovery audit.
- **haos-backup-parser-sanitizer-fuzz** : PR #140 head 8453d35a01a805a784252801cad49ecf371cbfaa merged 016c002da5ef7e529438ddf30203b7df281ac6fc, 11/11 green. Real native HAOS unprotected backup importer + nested gzip + entity registry + inert original automations fuzzed by pinned libFuzzer/ASan in GitHub run [40342730820](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40342730820), **263654 executions in 46s**, no sanitizer failure. Five synthetic tar corpus archives, no household. Separate extended manual workflow run [40343956190](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40343956190) completed SUCCESS at exact main SHA 016c002da5ef7e529438ddf30203b7df281ac6fc; this is a distinct extended laboratory run, not a penetration test. This is not an independent penetration test, whole-stack security audit or long soak.

Current release verdict is unchanged: NOT_PRODUCTION_READY, 0/15 independently certified.

## P8bi / P8bj — official HAOS disabled YAML to fake Moli, then durable approval revocation (Oct 26)

- **P8bi**: PR [#142](https://github.com/GodsQuantum/Rusty-Home-Open/pull/142), original head 2293d85b0d2c0c1ef514e2be75ec4f25f48b75f9, merge 6eeda5b718f3bd6b214053808e082841bd7494b9, **13/13** exact-head checks SUCCESS. Official isolated HAOS 18.3 Supervisor archive → native Rust private original disabled YAML → parsed one synthetic motion-to-light Moli graph, explicit fake test approval, fake actuation, rollback. Actual QEMU HAOS [40348951772](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40348951772). Not household automations parity or an independent human approval.
- **P8bj**: PR [#144](https://github.com/GodsQuantum/Rusty-Home-Open/pull/144), original head 06e6a977a2c45cea3d04ec124241c552d90dee8e, merge 0e16eac7f848d98fc0c16b5ce80b9a7c371f8faf, **10/10** exact-head checks SUCCESS. New actual upstream Moli test saves a modified agent action to automations.json, refuses the previous human-test fingerprint, proves no run/actuation after edit and no reactivation after reloading disk into a fresh Hub/engine. GitHub native Rust [40352628593](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40352628593); genuine official HAOS QEMU [40352628694](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40352628694) is a regression of the shared source importer, **not** a new household cutover proof. CT700 three targeted runs PASS, strict Clippy PASS. Source: crates/native-kernel/src/automation_moli.rs and docs/P8BJ_PERSISTED_MOLI_APPROVAL_REVOCATION_2026-10-26.md.

No physical radios, VM100, real service command, household credential or real human audit was used. These two receipts do not certify the 15 production gates, 72-hour endurance, full YAML translation, 29 integrations or HAOS↔Rust reversibility. Release remains **NOT_PRODUCTION_READY, 0/15**.

## 26 October 2026 — P8bk and P20 exact laboratory CI receipts (no production signoff)

| Lab proof | PR / exact original HEAD | Merge | Verified workflow/job | Required checks | Real verified behavior |
| --- | --- | --- | --- | --- | --- |
| P8bk bounded Core WS recovery | [#148](https://github.com/GodsQuantum/Rusty-Home-Open/pull/148) / `9c00276bc2140c4ee6b9f84d1c76f90d3bb8bb39` | `5e0abb94341b5a84be4e75a248f2d7c3c6058b8d` | [40362067016](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40362067016), job 121709861718 | 9/9 SUCCESS | Actual refused local TCP connection, then real hass-rs authenticated subscribe+snapshot in newer epoch; rejects old state, denied auth, exhausted retries; official Core WebSocket→Moli read-only reauth marker |
| P20 Linux TTY kernel exclusion | [#147](https://github.com/GodsQuantum/Rusty-Home-Open/pull/147) / `6743d139e08587787a22a1fc70b431a6f34092f7` | `2aa23c45d9938dc7cbab210eb3901d2a8d2bc258` | [40362790715](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40362790715), job 121712400095 | 9/9 SUCCESS | Actual Linux PTY TIOCEXCL excludes second open through SIGSTOP and SIGKILL while trusted descriptor survives; explicit TIOCNXCL releases; O_NOFOLLOW rejects symlink |

Read directly from job logs: `bounded_retry_denies_live_session_and_exhaustion_leaves_no_state ... ok`, `bounded_retry_survives_real_loopback_tcp_outage_with_new_auth_epoch ... ok`, `P8BH_REAL_CORE_WS_REVOKE_BAD_AUTH_NEW_EPOCH_NATIVE_HUB_PASS`, `P20_REAL_TIOCEXCL_SIGSTOP_DENIED_SIGKILL_STILL_LOCKED_EXPLICIT_RELEASE_PASS`, `P20_NO_USB_NO_RADIO_NO_VM100_NO_PRODUCTION_AUTHORIZATION_PASS`. These are real but narrow isolated laboratory checks. Neither certifies actual USB/Zigbee/Thread/Z-Wave ownership transfer, production authentication/TLS, 24–72h reconnect/soak, household cutover, legal signoff, nor independent pentest. Production release remains 0/15, NOT_PRODUCTION_READY.

## 26 octobre 2026 — P2i MQTT5 vrai broker SIGKILL, quarantaine redb [VÉRIFIÉ LAB UNIQUEMENT]

PR [#153](https://github.com/GodsQuantum/Rusty-Home-Open/pull/153), HEAD exact `6322f0607a80fd03e5ef190956d84e8384fd0149`, merge `45097f3a6cd9be085446e36c1076dc6ceb2e98a2`, **10/10 contrôles GitHub SUCCESS** sur ce HEAD. Nouveau workflow [40371604105](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40371604105), job `121743500539` `real-isolated-mqtt5-broker-sigkill` SUCCESS, logs de six marqueurs `P2I_..._PASS` vérifiés ; 4/4 tests unitaires Rust `mqtt_registry`, fmt et Clippy strict. Test local CT700 : 6/6 marqueurs et exit 0, temporaires et image Docker de laboratoire supprimés.

**Comportement nouveau réellement démontré :** véritable Mosquitto isolé `--network none`, config MQTT5 retained stockée en redb et projetée en Moli read-only ; broker SIGKILL pendant un abonnement actif ; redémarrage du même conteneur sans retained (persistence false) ; ancien redb présent mais `STALE_UNVERIFIED`, zéro périphérique/état Moli ; paquet de state sans nouvelle configuration retained rejeté ; re-publication config retained + state crée une nouvelle projection Moli read-only. Différent du P2g (SIGKILL writer redb) et P2h (tombstone hors ligne) : **ne pas rejouer ces preuves comme nouvelles**. Source : `scripts/test_p2i_mqtt5_broker_sigkill.py`, CI `.github/workflows/p2i-mqtt5-broker-crash.yml`, claim structurée `config/lab-evidence.json`.

**Limites :** aucune radio, périphérique réel, HAOS/VM100, broker du foyer, identité utilisateur, réseau externe au conteneur, autorité d'écriture, endurance 72h ou audit indépendant. Le vrai broker est tué dans le conteneur jetable, jamais un service de production. Produit `NOT_PRODUCTION_READY`, **0/15** gates signés.

## 26 October 2026 - P2j live upstream Moli same-ID tombstone/re-discovery [LAB ONLY]

PR [#155](https://github.com/GodsQuantum/Rusty-Home-Open/pull/155), original HEAD `7a5944e7e96838fefabd5305dbf415074dec4581`, merge `f6e5351f469716cca80cbfe34fcd0e97999d5769`, **10/10 GitHub PR checks SUCCESS** at exact HEAD. [Workflow 40380159515](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40380159515), job `121773908172` `actual-moli-live-tombstone-readd` SUCCESS: genuine upstream Moli Hub regression 1/1 PASS, native-kernel 55/55 PASS, strict Clippy and fmt. RED reproduced first: globally_known retained removed ID and a same-process re-add never registered. Fix releases only IDs owned by removed config. Real native Moli Hub clears state after removal, ignores old-topic packets, accepts a fresh same-ID discovery on a new topic with no inherited state, then updates only on new-topic packet; write permission always false. Structured proof `config/lab-evidence.json`. **No real MQTT broker in this new test** (prior P2h/P2i broker evidence is separate); no physical radio, VM100, household services, 72h soak or independent signoff. **NOT_PRODUCTION_READY, 0/15**.

## P2l — Moli malformed direct discovery batch [LOCAL LAB ONLY; CI PENDING]

LXC700 actual upstream Moli Hub targeted Rust test p2l_atomic_discovery_tests: RED 0/1 (malformed duplicate unique_id registered phantom, two devices instead of one), fix validates the entire in-process batch before mutating Hub; GREEN targeted 1/1 and native-kernel 56/56, Clippy strict/fmt/diff PASS. Also verifies rejection of cross-config identity theft and retained legitimate owner state, then explicit removal and fresh same-ID registration without state. Separate from P2j and pending P2k, no broker/radio/VM100, no independent certification. GitHub PR and exact-head CI pending; do not append an unverified claim to config/lab-evidence.json. Product 0/15.

## P2l — exact verified upstream Moli atomic discovery receipt (26 Oct)

PR [#158](https://github.com/GodsQuantum/Rusty-Home-Open/pull/158), original HEAD 6c2d061f251149b1a9afa1424a8971c218bf0b78, merge 6d13d8b61dfdc9ec606f70ef528a9f8b3a1a6593, **11/11 exact-HEAD checks SUCCESS**. Actual Moli CI [run 40389728103](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40389728103), job 121807245943 actual-moli-atomic-discovery-ingress SUCCESS, log confirms targeted 1/1 and native-kernel 56/56 PASS. RED before fix: malformed direct duplicate-ID Preview registered phantom Moli device (2 instead of 1). GREEN: atomic whole-batch validation, reject rival config identity theft and state-topic overwrite, reject rogue tombstone, legitimate removal/re-add no inherited state; all read-only. No actual MQTT broker in P2l (separate P2i/P2k), physical device, VM100, 72h soak or independent auditor. Structured claim moli_atomic_direct_discovery_batch_refusal remains verified_lab_only; production 0/15.


## P2k — actual MQTT5 retained tombstone and same-ID re-discovery (26 October 2026, LAB ONLY)

PR #157: exact original HEAD `bae1ee85596c7d2004bda91d2281b2a091c20303`, squash merge `56623b12a42ece6ceb6a148208e44d88d805529b`, **11/11 GitHub checks SUCCESS**. [Real isolated Mosquitto CI 40397842879](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40397842879), job `121835521396` at that exact HEAD. Its logs contain both distinct markers `P2K_GENUINE_MQTT5_BROKER_RETAINED_DELETE_READD_SAME_MOLI_ID_PASS` and `P2K_NATIVE_MOLI_OLD_STATE_CLEARED_NEW_VALUE_READONLY_PASS`; Rust MQTT registry 4/4. Real broker retained config -> old state 21.5 -> retained tombstone -> redb empty and Moli Hub zero devices -> SAME stable unique ID registered with no old state -> new state 31.2 in one read-only Moli point. Existing P2i broker-crash scenario was a setup/regression, not a second new proof. LXC700 reproduced both P2k markers with isolated Docker and no published ports. First PR head `eb21feba` had 10/11 checks due an unrelated transient QEMU synthetic registry SHA mismatch; after merge with main, exact final HEAD passed the unchanged strict HAOS SHA check. No assertion was waived.

Separate local portability correction: full Rust workspace initially failed one OCI App manager test under LXC700 `umask 0002` because its synthetic fixture used group-writable default permissions; explicitly private `0600` fixture permissions restore the actual manager security contract. After the test-only correction, **311/311 workspace tests**, strict workspace Clippy, fmt passed locally. This does not assert new real App behavior. No household, real broker, radios, VM100, human audit or production credential. **0/15 independent production gates**.

## P2n — durable redb MQTT discovery topic/identity ownership (26 Oct, VERIFIED LAB)

Scope only: actual owner-private on-disk redb, distinct from #161 in-memory Moli topic ownership. Initial failing Rust tests reproduced accepted cross-config duplicate state topic and accepted ambiguous legacy cache after restart. Three focused real redb regressions cover (1) same topic across two retained configs, (2) repeated topic inside one multi-component config, (3) pre-fix on-disk duplicate topic/identity recovery refusal without deleting original owner. Transactional refusal, explicit tombstone release, reopened records always stale_unverified. Exact original PR head, job and CI markers must be filled ONLY after verification. No production gate, no independent review or real broker in these NEW tests; existing P2i broker test is regression. Release NOT_PRODUCTION_READY, 0/15.


## P2m/P2n — original exact GitHub receipts, verified 26 October (LAB ONLY)

| Case | Original PR HEAD / squash merge | Exact GitHub run/job | Checks | New behavior only |
| --- | --- | --- | --- | --- |
| P2m in-memory real Moli | #161 c0b5cd4c963ca8f9ea1ca5162ec183ff53278909 / 0d6bbbace80e1b6923065d4108a47fc3be9b3712 | [40404859841](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40404859841), 121859885821 | 11/11 SUCCESS | Real upstream Moli refuses cross-identity and in-batch shared MQTT state topic; targeted test and P2M safety marker in CI logs |
| P2n persisted redb | #162 4837233c4870c6e0604d35a064f6d748c264591f / e50928759be7b8c07b882c1c7cca460bd3194301 | [40406964357](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40406964357), 121867096840 | 10/10 SUCCESS | Genuine owner-private redb 3/3 targeted tests: transactional duplicate topic refusal, legacy duplicate identity/topic recovery denied, valid tombstone/reopen; P2i/P2k actual Mosquitto rerun was regression ONLY |

The corresponding CI job metadata explicitly reports SUCCESS at the ORIGINAL feature HEADs and the logs contain the new test names and P2M marker. No real household broker, HAOS VM100, radio, live credential, independent pentest, release signing or 72-hour soak. Both are verified laboratory cases, never production gates. NOT_PRODUCTION_READY, 0/15.

## P8ax — independent restore of SECOND genuine stateful OCI App (26 October)

**Merged #164:** original feature HEAD 8a375a809b5ed1f078e42eb82ba9f6535bcca981; squash commit 2a2813dd5e96b87adac99048665f27adddbab792. **11/11 GitHub checks green**. Official original SHA run https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40409988909, job 121877915230, workflow P8aw OCI App custom volume path with genuine Docker Distribution registry storage.

Actual official Docker Distribution 3.1.2 under native generic rootless Podman App manager: original Registry stopped; state exported to private 0600 tar; SHA verified and corrupt archive refused. Original restarted and acquired NEW OCI blob after backup. Previously captured tar imported into a distinct fresh Podman named volume; new independently running Registry served old blob via localhost HTTP and returned 404 for post-backup blob. Original retains both. Restored App uninstalled without deleting its volume or disrupting source.

Scope: Redis (P8aq) and Distribution are TWO real independent stateful restores, not nine representative Apps, not arbitrary Supervisor database migration, not production cutover. No household HAOS/radios, no 72 h soak, no independent reviewer. Release remains NOT_PRODUCTION_READY (0/15) and supervisor_apps_restore lab_partial.

The same PR repaired GitHub REST 422 on giant old PR #59 using GraphQL only for the exact known REST error, requiring merged main branch and verified 40-character head/merge SHAs; 403/404/other errors still refuse. Exact online provenance check is not skipped.

## P8ay — third REAL stateful OCI App: PostgreSQL 17 independent SQL checkpoint restore (26 October)

- **PR #166 merged** as 848f2e480ec2c8d84aee2bdf0865ec574df2f0b0; original verified feature HEAD 1fd8101f5658338ae86bdecbb9a0ba8bfb3609f6.
- **GitHub exact-head run:** https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40413124727, job 121888597420, name Real rootless OCI Registry HTTP and PostgreSQL SQL separate-volume restores, workflow Generic rootless OCI Registry and PostgreSQL App persistence and restore. **20/20 checks on PR #166 SUCCESS** (15 successful workflow runs, 20 individual jobs).
- Official unchanged PostgreSQL 17 Alpine image pinned at its pulled digest, UID70 in rootless Quadlet, native /var/run/postgresql tmpfs Unix socket, network=none, read-only root filesystem. Bootstrap CI synthetic password never in persistent unit. Real SQL table pre-backup committed ONE row. Writer stopped before exporting owner-private Podman volume to SHA-256 checked 0600 tar; deliberate corruption rejected. Same image+generic Rust manager started original; a SECOND SQL row committed after checkpoint. A new independent named volume imported the original tar and started a SECOND PostgreSQL server concurrently. SQL SELECT confirmed original TWO rows, restored exactly ONE original row. Both App removal and volume/data preservation checked.
- Existing separate app proofs: Redis (P8aq) and Docker Distribution Registry (P8ax). Thus THREE DISTINCT real persistent Apps with independent restores, NOT nine required representative App lifecycles.
- No production passwords, household access, HAOS VM100, public socket, shared radio, 72h endurance or integration cutover. Still supervisor_apps_restore lab_partial and all 15 production gates NOT certified.

Source: scripts/test_p8ay_postgresql17_independent_restore.sh; native generic path sanitization in crates/native-hub/src/oci_quadlet.rs; no custom wrapper / hardcoded image install in the manager. This is independent exact SHA evidence for one NEW tested application.

## P8ca — real PostgreSQL 17 SIGKILL during open SQL transaction, WAL recovery (26 Oct, LAB ONLY)

- **PR #168 merged:** original exact HEAD 4299ca4d8c2a7d01fc471ad706b15c44d664f0cc; merge 9732d89f38ce2f91f358ce986b4059c520afa6c7. **11/11 GitHub checks SUCCESS.** [Real OCI run 40415028970](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40415028970), job 121895059779, workflow Generic rootless OCI Registry and PostgreSQL App persistence and restore.
- Actual official PostgreSQL 17 Alpine rootless, network none, UID70, read-only rootfs: third SQL row COMMITTED, fourth INSERT held UNCOMMITTED in an open SQL transaction confirmed by another real PostgreSQL connection; container SIGKILL, native Quadlet systemd Restart=on-failure and real PostgreSQL WAL replay. Four distinct P8CA PASS markers in the successful job log. Committed third row survives, uncommitted fourth row absent; second independently restored earlier volume remains at one row. No second fake daemon or manual app restart.
- RED attempts were NOT accepted: run 40414260249 failed on an obsolete final 2-row assertion after the new commit; run 40414782195 caught an official PostgreSQL transient init-server pg_isready race, fixed by requiring final PID1 postgres. Neither failure counted as a verified lab proof. Previous P8ay backup/restore and Registry tests are regressions, not novel receipts.
- **Scope:** one genuine PostgreSQL WAL crash/recovery, not nine-App lifecycle, real Supervisor App migration, host failover, 72-hour endurance, security audit or independent release. VM100/physical radios/production untouched. **NOT_PRODUCTION_READY, 0/15**.
