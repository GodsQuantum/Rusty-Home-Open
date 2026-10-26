# Rusty Home — handoff permanent

## ÉTAT CANONIQUE ACTUEL — 26 octobre 2026 (prioritaire sur les notes historiques ci-dessous)

- **P8be (#136) ET P8bf (#137) MERGÉS, nouveaux comportements prouvés** : entrée YAML HAOS réelle préservée inactive dans Rust (source officiel QEMU 40328174220, commit 004f2cb) ET fichier source restitué strict SHA256 dans une deuxième VM HAOS officielle, directement 40330463271 et après coffre Rust 40330463267 (commit 18b519e). Ces expériences ne prouvent pas l'exécution de YAML par Rust, mais **NE DOIVENT PAS être réessayées sans modification justifiée**. Détails/exact SHAs dans docs/CERTIFICATION_EVIDENCE_REGISTER.md et config/haos-native-backup-evidence.json.

- **Développement :** dernier HEAD confirmé avant réconciliation documentaire `main` `18b519e9b91451fed38a22665a9109ca7682225b` (actualiser via git status/log lors de chaque reprise), LXC700 (`/home/arezki/Projets/Applications/Rusty-Home-Open`), GitHub `GodsQuantum/Rusty-Home-Open`. Worktrees temporaires de la reprise nettoyés, branche principale synchronisée par fast-forward.
- **Production : `NOT_PRODUCTION_READY`, 0/15 gates certifiés** (9 `lab_partial`, 6 `blocked`). `python3 scripts/check_release_gate.py --matrix` est l'unique verdict courant. Une CI verte est une preuve LAB bornée, jamais une attestation indépendante ni une autorisation de basculement.
- **Fonctions vérifiées ce jour :** #127 (`b1881a6`, vraie Registry HTTP/volume), #128 (`e5968c2`, CI HAOS/OCI ciblée, 28/28), #129 (`df796cd`, matrice de 15 gates, 9/9), #130 (`075e18e`, vrai verrou OCI interprocessus, 18/18), #131 (`a351529`, secret Podman réellement monté, 20/20), #132 (`6ee9e7e`, 11 preuves OCI immuables, 9/9), #133 (`35b09f0`, futures certifications indépendantes acceptables une par une, 10/10), #134 (`16a1f49`, vrai Core 2026.10.4, 10/10).
- **Preuve Core récente à NE PAS REJOUER :** PR #134, head original `21d9b21421a215856930764f2264509f06459a3a`, GitHub Actions `40319204128` (succès), Core officiel 2026.10.4 conteneur isolé, config-flow/entry install, unload, reload, remove et version vérifiée. Premier run `40318906283` échoué sur un import erroné du fixture, corrigé via `homeassistant.const.__version__`. Core 2026.10.2 précédent et Core 2026.11-dev Marketplace restent des témoins séparés ; pas 29/29 HACS.
- **Priorités réellement manquantes :** migration complète réversible HAOS→runtime Rust→HAOS avec identités/automations/Apps et bases tierces ; RBAC/opérations utilisateur révocables ; vrais contrôleurs radios avec propriété exclusive ; 72 h de charge/soak ; licences, publication signée et pentest indépendant. Ne pas refaire sauvegarde HAOS même VM/seconde VM, OCI Redis+Registry ou test Core 2026.10.4 sans changement de code/environnement.
- **Interdits :** aucune mutation VM100 HAOS familiale, radios, secrets ou données réelles sans autorisation spécifique ; utiliser SentinelX→LXC700 uniquement, branche DEV propre, réutiliser les sources et les tests upstream.

---

**Audit du 24 octobre 2026.** La référence historique de base est `main` 6585cb4 ; elle ne représente pas nécessairement le dernier commit. Pour connaître le HEAD du dossier et les critères actuels : `python3 scripts/show_handoff.py`.

## État confirmé

Produit **NOT_PRODUCTION_READY** : 15 critères, **9 partiels, 6 bloqués, 0 totalement certifiés**. Noyau Rust natif encore synthétique et connexion Home Assistant principalement en lecture seule.

**NE PAS répéter comme nouveaux tests :** sauvegardes et restaurations HAOS complètes/chiffrées, refus du mauvais mot de passe, SHA-256 File Editor, rollback App, installation synchrone Supervisor et récupération HAOS→deuxième VM HAOS vierge (6/6). Lire le [registre](CERTIFICATION_EVIDENCE_REGISTER.md) avant toute tâche.

## Les sources de vérité

- [Audit exhaustif des 15 critères, des alternatives et des délais](PRODUCTION_CERTIFICATION_MASTERPLAN_2026-10-24.md) et [données structurées](../config/certification-masterplan-audit-20261024.json).
- [Preuves de laboratoire](CERTIFICATION_EVIDENCE_REGISTER.md) et [provenances JSON](../config/certification-evidence-registry.json).
- [Journal des travaux](WORK_LOG.md) et [gates de production](../config/release-gates.json).

## Chantier P8q du 25 octobre 2026

La première tranche d'import HA→Rust utilise la projection Moli existante, avec registre d'entités durable et rollback atomique entre processus. Voir docs/NATIVE_HA_ENTITY_REGISTRY_STAGE_2026-10-25.md. Les 338 tests Rust workspace et les tests de sécurité multi-processus sont verts en local. Le vrai Core→Rust doit être exécuté dans la CI GitHub avant toute affirmation de validation du scénario réel. Les états, automations et actions ne sont PAS migrés. Le produit reste NOT_PRODUCTION_READY, 0/15 critères certifiés. Ne pas dupliquer l'importeur ni les tests de restauration HAOS déjà prouvés.

## Étape supplémentaire P8r — vrai Hub Moli OS (laboratoire)

Le module native-kernel/ha_registry_shadow utilise le lot d'identités Core déjà importé par la PR #89 pour enregistrer des périphériques read-only dans le vrai Moli Hub, sans inventer leurs valeurs ni permettre des commandes. Les tests unitaires couvrent 1 232 entrées. Voir docs/NATIVE_MOLI_HA_REGISTRY_SHADOW_2026-10-25.md. **Attendre la CI Core réelle avant de certifier la chaîne complète.** Le prochain chantier restant est l'import fiable d'états réellement observés et des automatisations, pas la recréation du stockage d'identités.

## P8s — données d'états de Core vers Moli (instantané, laboratoire)

Après #89 et #90, l'étape suivante importe un instantané officiel State.as_dict() dans les mêmes identités Moli, sans inventer les états manquants ni autoriser des commandes ; voir docs/HA_CORE_STATES_TO_MOLI_SNAPSHOT_2026-10-25.md. Ne pas présenter cette nouvelle étape comme un abonnement WebSocket continu : il reste à faire la convergence temporelle, les sessions, les automatisations et la migration des données complètes. CI du vrai Core obligatoire avant certification.

## P8t — événements WebSocket vers le Hub Moli (25 octobre)

Nouvelle branche feat/ha-core-event-convergence-20261025. Module crates/native-kernel/src/ha_event_convergence.rs, documentation HA_WS_TO_MOLI_EVENT_CONVERGENCE_2026-10-25.md. Réutilise les PR #89/#90/#91 et le client hass-rs déjà présent ; vérifie changements d'états, timestamps, générations, pertes et révocation. Une fermeture silencieuse peut laisser une file apparemment ouverte : la sonde active get_states est indispensable. Preuve réseau : véritable socket loopback synthétique, PAS le serveur WebSocket du Core familial. Vérifier RustSec, CI complète et l'état de la PR avant fusion. Toujours 0/15 gates certifiés ; prochaines étapes : TLS/auth, surveillance périodique, automatisations et services, restauration complète HAOS→Rust→HAOS.

## P8u — exécutable de laboratoire Core WS → Moli

Après la PR #92, le module crates/native-kernel/src/ha_ws_runtime_lab.rs et le flag natif --lab-ha-ws-to-moli fournissent un runner à durée bornée : jeton via HARS_LAB_WS_TOKEN (jamais argv), URL loopback uniquement, entité Core déjà présente dans le registre, valeur attendue et nombre limité d'événements. Réutilise EventConverger, hass-rs et le vrai Moli Hub. Test serveur WebSocket réel mais fictif et commande lecture seule. Voir docs/HA_WS_LAB_RUNNER_2026-10-25.md. Pas de connexion au vrai WebSocket Home Assistant Core, ni de certification de service HAOS, production toujours 0/15.

## P8v — vraie authentification WebSocket Core officiel (25 octobre)

Nouvelle branche feat/real-core-websocket-to-moli-20261025 : scripts/test_p8v_genuine_core_ws.py (vrai Core 2026.10.2, auth utilisateur, registre, états), scripts/run_p8v_genuine_core_ws_to_moli.sh (Core Docker --network none, runner Rust dans le même namespace uniquement, aucun port hôte), scripts/test_p8v_official_core_ws_contract.py et job CI officiel. Lire docs/GENUINE_CORE_WS_TO_MOLI_2026-10-25.md. Réutiliser les PR #89–#93 et ne pas reconstruire leurs preuves. **À vérifier : CI réelle au SHA exact**. Compteur release 0/15 ; pas encore de migration HAOS productive.

## P8w — cohérence autoritative get_states/ShadowSession/Moli (25 octobre)

Nouvelle branche feat/ha-ws-state-consistency-20261025, dossier Rusty-Home-WS-Consistency-QA-20261025. Les PR #89–#94 sont déjà fusionnées : ne pas reconstruire le chemin réel Core→WebSocket→Rust→Moli. Le nouveau contrôle compare les valeurs issues du get_states officiel, du miroir authentifié et du Hub Moli. Une divergence efface tout et exige une reconnexion/re-snapshot. L'essai CI réel doit valider 8 événements consécutifs et le rejet auth_invalid par Core d'un jeton fictif erroné. Voir docs/HA_CORE_WS_STATE_CONSISTENCY_2026-10-25.md. La synchronisation des attributs, la reconnexion permanente, les automations et services, ainsi que HAOS→Rust→HAOS restent hors preuve. Release 0/15.

## P8x — historique de sept vraies preuves Core→Rust/Moli à préserver

Les PR #89 à #95 sont fusionnées. Consulter obligatoirement config/native-migration-evidence.json et docs/CERTIFICATION_EVIDENCE_REGISTER.md : leurs HEAD SHA, commits de fusion, runs CI exacts, scénarios et limites y figurent. Un nouveau contrôle scripts/test_native_migration_evidence.py empêche d'effacer ces preuves par accident. Les restaurations HAOS→HAOS (même VM et deuxième VM) sont dans un autre registre et NE SONT PAS à refaire. La réussite de vrais tests de laboratoire Core, WS, Rust et Moli ne valide pas encore la migration du foyer ni l'automatisation/services. Toujours 0/15 gates production ; ne pas annoncer la production.

## P8y — trou détecté sur mises à jour d'attributs (25 octobre)

Après #96, une comparaison Core/WS/Moli seulement sur le texte d'état peut laisser passer un state_changed d'attribut manqué. La branche fix/ha-core-same-state-gap-20261025 impose last_updated et les attributs JSON identiques pour les deux lectures authentifiées et invalide tout le Hub sinon. Test dédié ajouté. **Périmètre laboratoire, CI à vérifier ; reconnexion automatique, vrais attributs et migration HAOS→Rust restent ouverts. Production 0/15.** Voir docs/HA_CORE_WS_STATE_CONSISTENCY_2026-10-25.md et le journal.

## P8y — sécurité d'attributs fusionnée et vérifiée

La PR [#97](https://github.com/GodsQuantum/Rusty-Home-Open/pull/97) est fusionnée dans main au commit c8c6a7362f500517dfe2516bcb223d7839176f44. Le SHA de feature 141bf49f44cfb72e8ab36d0a6b47e2140033c78b a réussi les **9/9 contrôles GitHub**, dont vrai Core WS→Moli et Rust/sécurité. Ne pas répéter ce correctif ni l'import historique #89–#96. La certification de production reste 0/15.

## P8z — lecture cohérente malgré une course Core/WebSocket (laboratoire)

Le contrôle P8y peut révoquer une session saine lorsque l'événement Core arrive dans la file durant get_states. La branche fix/ha-core-ws-read-probe-race-20261025 draine avant et après chaque snapshot, puis ne retente qu'au plus deux fois. Elle n'autorise aucune lecture sans égalité d'état, d'attributs JSON et de last_updated entre Core, miroir authentifié et Hub Moli. Une perte permanente est révoquée; le nombre d'événements consommés par les sondes demeure comptabilisé par le runner de laboratoire. Test WebSocket loopback pour les deux trajectoires : événement retardé livré, événement véritablement perdu. Aucun support de reconnexion illimitée ou basculement du domicile; CI SHA exact obligatoire. Documentation : HA_CORE_WS_STATE_CONSISTENCY_2026-10-25.md.

## P8aa — voie HAOS full archive vers Rust CERTIFIÉE EN LABORATOIRE (25 octobre)

Main avant intervention a45e9d6 (PR #97/#98 fusionnées), nouveau worktree Rusty-Home-Certification-20261025. Nouveaux modules Rust d'import tar non chiffré vers registre durable, tests hostiles et workflow officiel HAOS 18.3 QEMU source→Rust sans VM cible et sans foyer. Préserver les preuves HAOS→HAOS et Core→Moli déjà acquises. Lire docs/HAOS_TO_NATIVE_BACKUP_STAGE_2026-10-25.md. CI QEMU RÉUSSIE : run 40238122942, 3/3 tests au SHA 9c6e3220f433153c0dc28ef0d2549bd52a345a8f, PR #99 fusionnée d297bbbd29ac313723b54794ad27aceaf0636166. Ne pas retester ces preuves déjà acquises ; reprendre les lacunes de migration réelle. Les Apps, automations, secrets et réversibilité système restent hors scope, 0/15 production.

## P8ab — opaque reversible Rust escrow (source HAOS->Rust->fresh HAOS)

New P8ab worktree Rusty-Home-P8ab-20261025 from origin/main e7235f7. See crates/interop-moli/src/backup_escrow.rs, scripts/test_p8ab_backup_escrow.py and .github/workflows/p8ab-native-escrow-rollback.yml. P8ab is VERIFIED: PR #101 merged as 4757377d5d773844f6e278af4712597d50cd799a after 12/12 checks, exact head 58cbfe3023532b7acca68123641a58692fab541d, real QEMU run 40240344194, source 3/3 and target 3/3. Machine evidence config/native-backup-escrow-evidence.json. The distinct P8ab test inserts a native, private, durable Rust archive vault between the already proven two official HAOS QEMU guest boots. Crucial: must await the exact P8ab CI success, no production 15 gate signoff, no actual Core or App execution. Preserve all earlier QEMU, Core and Moli proof IDs.

## P8ac — Rust orchestrates official HAOS guest (25 Oct)

New branch feat/real-haos-guest-supervisor-20261025 and separate DEV worktree Rusty-Home-Guest-Supervisor-20261025. Module crates/native-hub/src/haos_guest.rs, binary haos_guest_lab and official KVM workflow .github/workflows/p8ac-rust-guest.yml. Await exact GitHub CI SHA. This only proves authentic guest OS boot from a native Rust launcher, not healthy Core/Apps or production. Read docs/HAOS_RUST_GUEST_COMPAT_2026-10-25.md. All earlier #99-#102 data recovery proofs remain acquired. 0/15 production; do not touch family HAOS or pass-through radios.

## P8ad — real Supervisor and Core CLI health in Rust-managed HAOS

P8ac native Rust HAOS KVM serial boot was proven in GitHub run 40243098816, feature 29f9a45db72e4ef69aa0abed00dda17483d7133e, merged as ac76a85dbeadc1456968e5912eb7ddf32c9ba2a3 (PR #103, 18/18 checks PASS). The next P8ad branch feat/haos-rust-supervisor-health-20261025 and worktree Rusty-Home-Guest-Health-20261025 extend the Rust launcher to probe official Supervisor and Core via real HAOS guest shell, not merely serial OS boot. New CI .github/workflows/p8ad-rust-health.yml; wait for exact SHA result before merging. No household service or radios, 0/15 production.

## P8ae — guest state retained under native Rust (25 Oct)

P8ac PR #103 KVM boot and P8ad PR #104 real Supervisor/Core CLI result=ok already merged; DO NOT REDO those. P8ae branch feat/haos-rust-persistent-guest-20261025/worktree Rusty-Home-Guest-Persist-20261025 adds Rust-owned persistent official HAOS QCOW2 overlay, single-writer flock, genuine KVM guest marker persisted across abrupt QEMU process restart, official base SHA pinned. New workflow .github/workflows/p8ae-rust-persistent.yml. Its new independent two-boot CI at exact feature SHA is mandatory before certification; no family HAOS devices, full Apps compatibility or field rollback implied. 0/15 production gates.

## P8af — Rust process independently resumes HAOS guest (26 Oct)

P8ae PR #105 12/12 GitHub CI SUCCESS at feature 4534b23, merged 45811bcefa791544d7a016dc331d140fc585e120. Preserve this completed evidence; no repeat under P8af. Branch feat/rust-manager-recovery-20261026 in separate worktree Rusty-Home-Guest-Reattach-Main-20261026 adds verified_existing_overlay and two separate Rust manager commands to prove genuine HAOS marker retention across both manager and QEMU process restarts; separate CI p8af-rust-reattach.yml. Refuse duplicate owners, symlinks and altered QCOW2 backing path. Not production or real device ownership; 0/15 gates.

## Verified P8ac–P8af Rust host / genuine official HAOS guest (26 October)

PR #103 P8ac real serial boot 18/18, run 40243098816; PR #104 P8ad genuine official Supervisor+Core CLI health 11/11, run 40243728521; PR #105 P8ae persistent QEMU guest disk hard restart 12/12, run 40247000475; PR #106 P8af separate Rust manager disk reattachment and double-owner denial 13/13, run 40247834775. ALL FOUR MERGED on main: d8c5bee817c7b7e8ef5ea6af24d62d366fbe610a. Exact feature SHAs, run markers and limits in config/haos-rust-guest-evidence.json and docs/HAOS_RUST_GUEST_COMPAT_2026-10-25.md. These official HAOS 18.3 isolated KVM proofs are COMPLETE; DO NOT RE-RUN by hand or substitute for production certification. Next nonredundant gaps: installable long-running Rust guest host service/secure networking and Core/API+Apps lifecycle; independent proof of real device/radio ownership, full rollback and 72h reliability soak. Zero production gates certified, never touch household HAOS.

## P8ag native guest host service lifecycle — 26 October

Branch feat/native-guest-service-lifecycle-20261026, isolated worktree Rusty-Home-Lifecycle-20261026 from main 7957fce. New Rust guest host service and executable haos_guest_service_lab; opt-in laboratory QMP and lifecycle CI .github/workflows/p8ag-guest-service.yml. Prior P8ac-P8af proofs pinned and merged; don't repeat. No real home cutover or deployment, 0/15 production. Validate exact SHA in CI before marking verified.

## P8ah — loopback-only Rust guest HAOS frontend transport (26 October)

PR #108 P8ag genuine Rust service lifecycle passed 14/14, merged 0d943526a1e62f91658d92db1b88aea947958a7c. New branch feat/haos-local-core-ui-20261026 in worktree Rusty-Home-Loopback-20261026 exposes only optional fixed loopback QEMU host TCP/18180 to guest HAOS TCP/80, lab-only. Genuine P8ah KVM CI required; its TCP-listener proof does NOT yet demonstrate complete UI/API/Apps availability. No family HAOS deployment, 0/15 gates.

## P8ai — actual official Core frontend response from Rust (26 October)

P8ag PR #108 (14 checks; run 40250349459, main 0d943526) and P8ah PR #109 (15 checks; run 40251693506, main 8cebcf788cde7b18dd893d28fb6d3d75abb97e51) are MERGED and must not be retested as new proof. P8ai branch feat/rust-haos-real-frontend-health-20261026 in worktree Rusty-Home-Core-Frontend-Main-20261026 adds a real Rust HTTP GET / check against the fixed local HAOS guest 80/tcp route; new official KVM workflow p8ai-real-core-frontend.yml must pass exact feature SHA. This only proves the uncredentialed frontend responds with HTTP success/redirect; product remains not production-ready (0/15), no household HAOS/radios.

## P8aj — actual systemd managed HAOS guest supervision (26 October)

PR #108 genuine Rust HAOS guest service 14/14 merged 0d943526, #109 local guest UI 15/15 merged 8cebcf7, #110 Core HTTP 16/16 merged e891401d65d6f954afb809bd7ff9d7312c7b3e7f. All verified; NEVER retest as new proof. New branch feat/native-haos-systemd-supervisor-20261026 and separate worktree Rusty-Home-Systemd-QA-20261026 add opt-in systemd sample unit and scripts/test_p8aj_real_systemd_guest.sh. GitHub .github/workflows/p8aj-real-systemd.yml must genuinely start/stop the official HAOS 18.3 VM twice from systemd with full owner/KVM isolation at exact PR SHA before certification. No family HAOS/reboot/radios, 0/15 production.

## P8ak — native Rust guest Debian lab distribution (26 October)

P8aj #111 real official HAOS 18.3 under genuine systemd: 10/10 CI, GitHub run 40254861417, merged 4a1377341eabd44bc81334705465a5ebf33cf6c4. Certified lab only; DO NOT REDO.
New separate branch feat/haos-rust-guest-debian-package-20261026, worktree Rusty-Home-Guest-Package-20261026. Changes the LAB unit executable path to Debian policy /usr/libexec/rusty-home; new rootless reproducible .deb builder scripts/build_guest_debian_package_lab.sh, actual Ubuntu dpkg upgrade/downgrade/purge workflow .github/workflows/p8ak-guest-debian.yml and updated real KVM systemd test P8aj. CI at exact SHA required. Package postinst creates only dedicated sysuser; no service start/enable or family deployment. Still no signed general host release, full Apps or hardware radio takeover; 0/15 production.

## P8al — signed APT metadata for native Rust HAOS guest (26 October)

P8ak #112 exact-feature GitHub checks all 11 PASS; genuine package install/update/downgrade/purge run 40255870443, official HAOS systemd KVM run 40255870364; merged cae2cde2ce1e08eb86bd5a2b0465212be07845cf. DO NOT REDO. Current branch feat/signed-native-haos-guest-apt-20261026, worktree Rusty-Home-Signed-Guest-APT-Main-20261026. Extends existing signed APT fixture scripts/test_signed_apt_lab.sh with native guest versions v0.1.0 and v0.1.1 in same ephemeral signed Packages index; offline Ubuntu apt-cache both versions, deliberately tampered InRelease rejection. Independent static contract in scripts/test_p8al_signed_guest_contract.py. Pending exact-head GitHub public-debian-apt.yml job. No production signing key, no auto-start, no Cloud9 family or radio changes. 0/15.

## P8ao — generic OCI Apps via upstream rootless Podman Quadlet (26 October)

P8am #115 is merged on main 01265a9; P8an #116 separate real QEMU SIGKILL crash-restart CI is active; DO NOT REIMPLEMENT here.
Branch feat/generic-oci-quadlet-apps-20261026, isolated worktree Rusty-Home-OCI-Quadlet-20261026, introduces generic OCI sha256 digest pinned App manifests compiled into official Podman Quadlet rootless configuration. No hardcoded App-name allowlist; constrained quotas, volumes and no auto-install. Official generator CI p8ao-oci-quadlet.yml is mandatory before even limited lab certification. This does NOT demonstrate real Apps running, login, backup or full Supervisor parity. Production 0/15.

## P8ap — genuine rootless OCI App systemd lifecycle + persisted data (26 October)

P8ao PR #117 generic arbitrary OCI digest-pinned rootless Podman Quadlet generator 20/20 CI green, run 40264974609, MERGED b8d322bc8c37e3c4302f7eb08e45c76dbebccd7b; DO NOT RETEST AS NEW PROOF. Current worktree Rusty-Home-OCI-Real-App-Main-20261026, branch test/real-oci-app-lifecycle-main-20261026, extends generic Rust OCI App manifest to select a bounded nonroot numeric container UID and native Podman :U volume ownership; NO hardcoded app name mapping.
Distinct CI p8ap-real-oci-app.yml uses official Redis 7.4-alpine image pinned dynamically to its actual digest and attempts real systemctl --user start/stop/restart of generated Quadlet on an isolated Ubuntu runner; checks redis-cli state persisted across restart, no host port, read-only root and exclusive rootless OCI runtime. This requires exact GitHub CI at head. Redis is a real App test, NOT blanket Supervisor add-on compatibility; no household deployment, credentials, hardware, or radio. Production 0/15.

## P8ar — native Rust manager controls actual OCI Apps under systemd (26 October)

P8ap #118 real Redis rootless App persistence 21/21 green merged 071b85bac4f887843be3a2f47d25e44ddcc03bd1 and P8aq #120 independent offline App volume restore/tamper negative 11/11 green merged 7cf1d3ad3fb74d9e7d8537f171e500b94d86eac0 are complete, DO NOT REDO.
New separate branch feat/native-oci-app-manager-lab-20261026 worktree Rusty-Home-OCI-Manager-20261026. Native lab Rust CLI crates/native-hub/src/bin/oci_app_manager_lab.rs is not simply a descriptor generator: it actually manages generic SHA-pinned pre-pulled OCI Apps through native Podman and systemctl --user install/start/stop/remove. Validates ownership/symlinks, no auto-start, nonroot UID, avoids deleting saved App state volume. CI .github/workflows/p8ar-real-oci-manager.yml genuinely proves Redis commands, persistent data and invalid inputs. Until exact-head CI this is uncertified new code. No network, household hardware, production RBAC or HA migration; 0/15 release.

## P8aq — genuine rootless OCI App backup into independent restore (26 Oct)

P8ap #118 real Redis rootless systemd App persisted data test 21/21 CI merged 071b85bac4f887843be3a2f47d25e44ddcc03bd1. DO NOT REDO. Current branch feat/oci-app-backup-restore-20261026, worktree Rusty-Home-OCI-Backup-20261026. Extends proven real Redis Quadlet lab script with HARS_VOLUME_RESTORE_LAB=1, native Podman volume export/import, sha256 integrity and corrupted-archive negative, new empty volume and separate Rust-generated Redis restore instance. Compares older backed-up data vs newly mutated original. CI .github/workflows/p8aq-real-oci-restore.yml PASSED 11/11, merged 7cf1d3ad3fb74d9e7d8537f171e500b94d86eac0. This is one REAL App volume scenario, not complete Supervisor migration, signed backups, 72h or production. 0/15.

## P8as — explicit localhost TCP for rootless OCI Apps (26 October)

P8ar #121 native Rust App install/start/stop/remove real Redis 23/23 green merged 5af8dc1a268c7316bfe99eb50290f94726a57541, run 40291069681; DO NOT REDO. New branch feat/oci-app-localhost-tcp-20261026, separate worktree Rusty-Home-OCI-Loopback-20261026. Builds on P8ar with generic AppNetwork Offline default or operator opted LocalTcp 127.0.0.1 only (never wildcard), native rootless Podman slirp4netns and PublishPort. Rust manager accepts --local-tcp 18879:6379 in disposable CI. Real P8as CI must verify Podman port 127.0.0.1:18879 and connect via host socket, no public binding. Critically slirp4netns permits outbound egress: not an outbound network firewall and not production remote auth. No household changes. Product 0/15.

## P8at — OCI digest switch and rollback (26 Oct)

P8ar native OCI manager #121 23/23 checks merged 5af8dc1 and P8as localhost-only ingress #122 24/24 checks merged 95123e0. DO NOT REDO. Branch feat/native-oci-app-rollback-20261026 and separate worktree Rusty-Home-OCI-Upgrade-20261026 add Rust manager explicit digest upgrade and manual rollback. Only pre-pulled sha256 images; old digest compare-and-swap; per-App private file lock and owner-private copy; atomic manifest swap; systemd restart and automatic previous manifest recovery on startup failure; preserve Podman named volume and refuse uninstall with pending rollback. Genuine Ubuntu CI p8at-image-rollback.yml compares Redis 7.4 Alpine and Bookworm images, image IDs and persistent state after switch and rollback. Exact head CI MANDATORY. NOT a universal application-specific healthcheck, transactional database schema rollback or production release. Family HAOS untouched. 0/15.

## P8au — generic native OCI healthcheck and automatic unhealthy-image rollback (26 Oct)

P8at #124 exact SHA f11a802 genuine Redis image switch and explicit rollback CI run 40295014593 PASSED. Its remaining unrelated HAOS/DEB regression gates must finish before merge. Current branch feat/oci-app-healthcheck-rollback-20261026 isolated worktree Rusty-Home-OCI-Health-20261026 builds on #124, adding operator-configurable safe native Podman Quadlet HealthCmd (non-shell-injected executable+args), HealthInterval/Timeout/Retries, bounded native podman healthcheck run after image swap, and automatic previous image recovery if the new App starts but fails the healthcheck. CI .github/workflows/p8au-app-health-rollback.yml must prove real healthy Redis upgrades and deliberately incompatible BusyBox image rollback with persistent data untouched. Existing OCI App, backups and HAOS proofs must not be repeated as new proof. Do not treat rootless Redis as universal Supervisor/App compatibility. No household HAOS deployment; release 0/15.

## P8av — two genuinely different OCI engines under same generic Rust manager (26 Oct)

P8au PR #125 native Podman healthcheck and automatic unhealthy-image rollback 26/26 CI MERGED at 05008d4cb9d2d577bec1567ea3bcda67f1faaa2e. Do not reimplement or rerun old certification as new work. P8av uses isolated worktree Rusty-Home-OCI-MultiApp-20261026, branch test/native-generic-multiapp-lifecycle-20261026 from that exact main. CI .github/workflows/p8av-real-two-oci-apps.yml runs TWO distinct real official OCI services, Redis 7.4 Alpine and Memcached 1.6 Alpine, simultaneously under UNCHANGED native Rust manager commands. Exact runtime digest pinning, two different nonroot image UIDs 999 and 11211, no automatic start, no host interfaces, read-only root; verifies Redis disk persistence, Memcached actual in-container text TCP protocol/ephemeral cache semantics, stops/restarts/uninstalls one without affecting other, two named volumes retained. New lab certification NOT green until head CI succeeds. Test names are fixtures, not fixed supported-App whitelist. No host-family HAOS, radio or production switch. Production 0/15.

## P8aw — safe custom named volume path for generic stateful HTTP OCI Apps (26 Oct)

P8av #126 genuine simultaneous Redis + Memcached independently managed as two actual OCI engines 10/10 exact SHA CI, merged 6ed03afa6d9a55a28cd4a51d28d48c21480fe57d. DO NOT REDO. New isolated branch feat/oci-generic-data-mount-20261026, worktree Rusty-Home-OCI-DataMount-20261026 extends existing generic OciApp with render_at("/absolute/container/path") using only a native Podman-managed named volume (no host bind). Fail closed on path traversal, injection, invalid absolute path and mounts over /etc,/root,/run,/proc,/sys,/dev. Native Rust app manager accepts --install-path-lab slug pinned_image UID absolute_data_path hostport:containerport under original CI-only authorization/ownership guard. CI .github/workflows/p8aw-custom-volume-registry.yml proved genuine Docker Distribution Registry 3.1.2, actual HTTP v2 blob upload to /var/lib/registry in custom named volume, verified GET after systemd service restart and localhost only port, non-root user, no autostart, readonly root. This is third unrelated OCI workload and first genuine stateful HTTP service with nonstandard storage path. All 28 checks passed at exact head be76a29; PR #127 merged 26 October 2026 as b1881a6. Reuse this evidence; do not rerun it without code or environment changes. Never deploy to Cloud9 household HAOS; no production RBAC/third-party HA Supervisor parity; 0/15 gates.

## P8ax — avoid redundant HAOS KVM CI when only OCI manager changes (26 Oct)

P8av #126 simultaneous real Redis+Memcached 10/10 GH CI merged 6ed03afa6d9a55a28cd4a51d28d48c21480fe57d; P8aw #127 stateful HTTP Registry and custom data volume: genuine test run 40301611933, 28/28 checks passed; merged on main as b1881a6. Laboratory proof only, not production certification.
This separate isolated worktree Rusty-Home-CI-Scopes-20261026, branch chore/ci-native-hub-path-scopes-20261026, fixes a major certification throughput defect: all 18 workflow pull_request file triggers previously had crates/native-hub/** and re-ran HAOS official KVM for OCI-only controller changes, and Redis OCI checks for HAOS-only code changes. New precisely scoped paths split 10 genuine HAOS VM jobs and 8 OCI App jobs; shared Cargo.toml, Cargo.lock, build.rs, lib.rs, tests/** trigger BOTH. Every job remains manually dispatchable, and the original jobs/assertions/real tests are not weakened or removed. Contract scripts/test_p8ax_scoped_ci_contract.py enumerates all 18 workflows and verifies isolation + shared coverage. One-time CI reruns all changed workflows. This speeds subsequent native OCI feature PRs but does not claim production gating, still 0/15.

## Reprise obligatoire

1. Lire ces sources, vérifier le SHA actuel de GitHub, les PR ouvertes, les preuves et la CI avant de relancer des travaux.
2. Utiliser une branche et un worktree DEV isolés ; **ne pas modifier Home Assistant familial**.
3. Prioriser un moteur Rust + OCI et le vrai Core/HACS comme voie de compatibilité généraliste. Tester la migration HAOS→runtime Rust ; ne pas refaire HAOS→HAOS.
4. Faire progresser authentification réelle/permissions, restauration Rust et rollback, Apps/driver radio physiques, endurance 72 h, publication signée, audit indépendant.
5. Dans chaque PR substantielle, actualiser **WORK_LOG.md, ce handoff et les preuves pertinentes** ; vérifier tous les contrôles GitHub sur le SHA final avant fusion.

**Cadence observée au 24 octobre :** 76 PR fusionnées (8 le 22, 50 le 23, 18 le 24 octobre UTC) ; médiane création→fusion 9,0 minutes. Ce nombre ne traduit pas une certification de production. L’objectif universel interdit toute liste blanche limitée aux 9 Apps ou 29 intégrations d’essai.

## Immutable verified P8ag–P8al Rust host evidence (26 Oct 2026)

- P8ag PR #108: 14/14 exact feature-head GitHub CI, run 40250349459; long-lived Rust/QMP lifecycle.
- P8ah PR #109: 15/15 exact feature-head GitHub CI, run 40251693506; fixed 127.0.0.1 UI forwarding.
- P8ai PR #110: 16/16 exact feature-head GitHub CI, run 40252930987; real official Core HTTP frontend.
- P8aj PR #111: 10/10 exact feature-head GitHub CI, run 40254861417; real systemd two HAOS/QEMU process lifecycles.
- P8ak PR #112: 11/11 exact feature-head GitHub CI, run 40255870443; reproducible Debian guest package real dpkg lifecycle.
- P8al PR #113: 9/9 exact feature-head GitHub CI, run 40257162323; offline signed APT index and tamper refusal.

All six are MERGED, latest main a756696b60081d641ad7ec9cf43144f3361a201b. Do NOT rerun as newly unproved work. Machine-readable source config/haos-rust-guest-evidence.json keeps each exact original feature SHA, merge SHA and workflow marker source. P8ag–P8aj official HAOS 18.3 QEMU/KVM is an isolated genuine guest; P8ak real Ubuntu dpkg and P8al cryptographic offline APT use disposable CI runners. No household deployment, no signed production trust key, no actual App service parity, no physical radios, no whole-host backwards cutover or endurance qualification. Production remains **0/15** fully certified.

## P8am — systemd native READY/watchdog handoff (26 Oct 2026)

P8ac-P8al actual certified isolated tests and exact-SHA immutable record are complete, P8ag-P8al consolidated in PR #114 merged cd22ee3c1e1f7039306468043f862e0509a7a421. New isolated branch feat/native-systemd-ready-watchdog-20261026 in worktree Rusty-Home-Notify-QA-20261026; pinned sd-notify 0.5.0, Type=notify (not Type=exec), READY only after genuine private QMP greeting, WATCHDOG only while QMP alive, STOPPING on shutdown. Extends official genuine HAOS 18.3 P8aj KVM CI with two WatchdogTimestampMonotonic positive progress checks and verifies QEMU reap. P8ak Debian lifecycle must also pass. None of this proves HA Core/App readiness or unattended production deployment. Never install on family HAOS/Cloud9. 0/15 release gates fully certified.


### P8am live CI diagnostic and source-license bookkeeping (2026-10-26)

- At first PR #115 head 304c848, official HAOS systemd Type=notify test timed out before READY. Root cause: the new QMP detection used literal escaped quote bytes instead of normal JSON quote bytes. Readiness now calls the SAME QMP greeting parser as the previously certified shutdown path; new regression fixture rejects double escaping, Rust 16/16 and strict Clippy pass. Re-run CI on new exact head; do not claim systemd READY verified before it passes.
- A newly pinned native crate sd-notify 0.5.0 increases Cargo.lock to 227 resolved packages, 215 registry dependencies, 219 external and 213 with bundled license/notice source. Regenerated unsigned SPDX 2.3 source graph (227 packages, 526 edges) and updated six historical upstream license receipts ONLY to the new Cargo.lock SHA. Legal redistribution/signed product compliance are still NOT certified.

## P8an — real automatic restart after QEMU SIGKILL (26 October)

P8am #115 Type=notify, QMP watchdog and two successful genuine official HAOS 18.3 systemd cycles (26/26 checks, run 40261132205) merged 01265a99a8548274fd454fdc81b5037ff299c60e. These earlier checks are complete and must not be duplicated as new evidence. New branch feat/systemd-qemu-crash-recovery-lab-20261026, worktree Rusty-Home-Crash-Restart-20261026. New opt-in GitHub disposable-only crash test sends SIGKILL to QEMU and requires new Rust MainPID via real systemd Restart=on-failure, existing disk inode unchanged, clean guest reap and immutable official HAOS source. New KVM workflow p8an-crash-restart.yml must be GREEN at exact SHA before cert. No family installation or radio pass through. Product 0/15 fully certified.


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

## 26 October 2026 — individual production gate truth display

The already-existing 15 gates remain the only release authority; `scripts/check_release_gate.py --matrix` adds per-gate laboratory state, production verdict, mandatory independent scenarios and remaining blockers without repeating HAOS restores. Never equate 28/28 CI to 15/15 production. PR #127 merged as b1881a6; PR #128 CI path scoping under review.

## 26 October — P8ay OCI lifecycle per-App lock IN PROGRESS

Ownership: SentinelX Cloud9 / LXC700, isolated branch fix/oci-app-lifecycle-lock-20261026, HEAD e5968c2. Last existing P8ar-P8aw genuine Redis/Memcached/Registry lifecycle tests were green; do not repeat them as new certification. New gap found: OCI install, start and stop are not protected by the same existing per-App flock as image switch/rollback/remove, so concurrent commands can race; add install-safe exclusive lock and lock both run-state changes. Preserve App units, data volumes and existing commands; regression test contention on genuine separate processes. No host/HAOS100/production access. Rollback: revert isolated PR; release gates unchanged.

## 26 October P8ay — lifecycle fencing delivered for CI qualification

All native OCI install/start/stop/switch/rollback/remove commands now reuse one persistent owner-private per-App flock. New tests in the existing Redis CI hold that actual inode across separate manager processes and check start/install denial, unchanged unit/data, successful retry, and reuse of same inode after remove/reinstall. DEV: 8/8 targeted Rust binary tests and strict Clippy passed. GitHub independent OCI CI on the new SHA remains required; until then this is NOT production-ready and 0/15 gates.

## 26 October P8az — native Podman per-App secret references [IN PROGRESS]

Owner: SentinelX on Cloud9, LXC700 only, isolated branch feat/oci-native-secret-ref-20261026, base main 075e18e. P8ay native lifecycle locks 18/18 CI already merged; do not repeat as new independent proof. Current gap: OCI Apps have state, restart and health but cannot consume externally provisioned credentials, limiting real third-party App parity. Implement strict opt-in Podman native Secret= with only validated preexisting rootless secret name, no secret content in source/untrusted frontend, no home secrets, and CI disposable Redis image mounting synthetic secret; preserve all previous default OCI isolation. Rollback: revert this branch. Release 0/15.

## 26 October — P8az implementation + CI pending

Generic native Quadlet Secret= validates lowercase opaque names only; privileged secret storage and material never enter Rusty Home. New CI uses independently provisioned fictional Podman secret, verifies SHA256 bytes mounted inside real rootless Redis, rejects nonexistent/malicious names and preserves rootless store after service uninstall. 23 core Rust and 8 native manager tests + strict Clippy passed LXC700. CI exact HEAD required. 0/15 production gates.

## 26 October P8ba — evidence registry reconciliation RUNNING

Owner SentinelX Cloud9 LXC700 on branch docs/oci-evidence-p8at-p8az-20261026, base main a351529. Existing immutable config/native-oci-app-evidence.json ends at P8as, omitting SIX verified distinct features already merged: P8at #124, P8au #125, P8av #126, P8aw #127, P8ay #130, P8az #131. Restore historical exact head/merge/run/check and scenario marker IDs from existing GitHub job proofs without rerunning any QEMU/OCI workload, update validator and central evidence/handoff/log. No host app data altered. Rollback: revert docs-only commit; 0/15 production remains unchanged.

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

## 26 October P8bb — unblock legitimate per-gate certification transitions [IN PROGRESS]

Owner SentinelX Cloud9/LXC700, branch fix/incremental-release-gate-tests-20261026, base main 6ee9e7e. Gap: check_release_gate.py supports proof-scoped independently signed promotion one at a time, but CI scripts/test_public_certification_ledger.py, test_release_gate.py, test_release_gate_evidence_truth.py and historical masterplan/evidence tests hardcode the old 9/6/0 snapshot and reject any future certified gate. Preserve historical baseline evidence, use live audit for actual current release and strictly reject unverified certification, keep deployment_authorized and release_signoff false; 0/15 currently. Rollback: revert branch. No changes to household/service/data.

## 26 October — P8bb gate transition hardcode corrected

CI now compares the CURRENT release gate manifest and its independently verified Ed25519 evidence instead of freezing all five current check suites at historical 9 partial/6 blocked/0 certified. The 24 Oct masterplan and HAOS backup receipts remain immutable snapshots. Signed-scope attestation negative and positive test already verifies that a real single approved gate can count without granting deployment or signoff. Local validation of release gate, ledger, historical evidence, masterplan and attestation passed. No real independent production certificate has been issued; current 0/15, no household changes.

## 26 Oct P8bb — remaining historical suites migrated to live proof authority

Four ancillary suites (historic source-licence receipt, fuzz CI, signed APT, genuine HAOS archive escrow) no longer freeze live gate statuses; they use check_release_gate.audit for per-criterion independently signed promotion, while preserving their immutable laboratory attestations. Targeted old-license and fuzz transition tests + complete APT/escrow and public gate/attestation tests PASSED locally. Inactive legacy Codespaces suite refers to files missing in public repository; it is unchanged, not a new certification claim. CI exact final SHA required; release 0/15.

## 26 October P8bd — official Core 2026.10.4 isolated ABI lifecycle [IN PROGRESS]

Owner: SentinelX Cloud9 LXC700, isolated branch test/core-2026-10-4-20261026, base main 6ee9e7e. Current upstream official Core stable 2026.10.4 (released 24 October) Docker GHCR OCI index sha256:23f4dfbb69ce76315dbc4de7915504f1c2dcecd7ca2648113a493a385972e133 checked with docker buildx manifest only. Existing verified 2026.10.2 baseline and 2026.11-dev marketplace lab remain immutable, no rerun as new proof. Add separate disposable CI using same native synthetic Core config-entry lifecycle with dynamically selected strictly pinned 2026.10.4 candidate, assert actual runtime version and full create/unload/reload/remove. No HAOS family VM100 data or drivers. Rollback: revert branch, leave current release 0/15.

## 26 Oct P8bd — Core 2026.10.4 candidate implemented, CI mandatory

The independent Core 2026.10.4 GHCR OCI index was verified upstream; new opt-in P8bd GitHub workflow runs actual Core config-entry create/unload/reload/remove and runtime version verification without changing original 2026.10.2 pinned baseline or Marketplace dev. Strict external, network-none, read-only, nonroot fixture continues. GH exact-SHA proof still required; no current 29-integration hardware/API parity, release 0/15.

P8bd CI first exact head 2854e3c failed: the new version assertion incorrectly imported `__version__` from top-level homeassistant (ImportError). Existing already proven Core scripts import from `homeassistant.const`. Corrected to same official API; this is a fixture import error, NOT yet a Core interoperability failure. New head must run real 2026.10.4 again; older 2026.10.2 baseline remains untouched.

## 26 Oct 2026 — automation source preservation increment RUNNING

Owner SentinelX Cloud9 / LXC700, branch feat/haos-automation-opaque-stage-20261026, base f675c96. Examined existing genuine HAOS→Rust entity-registry stage P8aa and native automation_yaml/Moli synthetic compiler P3a/b. New distinct gap: actual automations.yaml source from an unprotected full HAOS archive is silently ignored; preserving byte-exact original in the existing private redb with source-archive+registry binding enables audit and eventual reversible migration without executing it or inventing services. Implement safe bounded archive extraction, atomic registry+YAML staging, exact offline readback/negative tamper scenarios, and CI. No real devices/secrets/HAOS VM100 touched; original immutable P8aa+HAOS QEMU proof NOT repeated as new proof. Rollback revert feature branch. 0/15 independent production gates.

## 26 Oct 2026 P8be — actual disposable HAOS automation source fixture, CI pending

The original P8aa real Supervisor backup QEMU scenario now includes an inert, **invented** YAML automation in the official guest Core directory before generating the same full archive; readback from a separate host Rust process checks SHA-256, byte size, private inactive state, archive+registry binding and rollback without executing it. The SHA from the generated real official archive must match that fixture. Original P8aa genuine-VM proof will be rerun because its archive contents and Rust parser actually changed, not as redundant certification; only the new P8be marker represents a new behavior. GitHub real runner/PR evidence pending, 0/15 production independent signoffs.

## 26 October P8bf — real HAOS automation restore via Rust escrow [IN PROGRESS]

Owner SentinelX Cloud9 LXC700, branch test/p8bf-automation-roundtrip-20261026, base main 004f2cb. Existing P8aa original Supervisor backup with inert automations fixture P8be passed GitHub run 40328174220; P8ab full HAOS→Rust escrow→fresh second HAOS and direct P8p backups already passed with synthetic marker. Missing distinct proof: after independent fresh-target HAOS restore, automation YAML source bytes remain exactly original. Extend existing upstream real QEMU two-guest scenario to write invented inert YAML into source and SHA-256 compare on new target, with no household access. Changes to existing scenarios justify rerun; no duplicate test infrastructure, 0/15 release.

## 26 October 2026 P8bf — new independent round-trip verification on official HAOS KVM

The already proven source→second HAOS and source→native Rust escrow→second HAOS scenarios now assert that the original invented, disabled automations.yaml is byte-for-byte restored on the independent clean target AFTER official Supervisor full restore. A fresh target may not contain the fixture beforehand. The original 2-guest boot, Supervisor/Core health, full tar SHA-256 and VM-identity JUnit gates are unchanged. Scope: *original automation file custody/restoration only*, not runtime translation, activation, custom HACS parity or production cutover. CI exact SHA pending. 0/15 production.

## 26 October P8bg — immutable HAOS automation evidence registration RUNNING

Owner SentinelX Cloud9 LXC700 branch docs/p8be-p8bf-immutable-evidence-20261026, base main 18b519e. PR #136 head 3527be5 real Supervisor QEMU source run 40328174220 and PR #137 head ed07cc7 direct second HAOS run 40330463271 and native escrow second HAOS run 40330463267 passed, both PRs merged with 12/12 CI. Register them as scoped new historical laboratory cases in existing config/haos-native-backup-evidence.json and supporting validator; update same canonical handoff and evidence register. Preserve original P8aa frozen facts, no new app data or real machines, 0/15 production. Rollback revert docs commit.

## P8bh 2026-10-26 ACTIVE — Genuine Core WS resync after explicit revoke; reject invalid auth; new epoch/new Moli, reuse existing pinned Core CI. Owner SentinelX CT700, branch feat/real-core-ws-reauth-recovery-20261026, base bd7353b. No VM100/hardware. Rollback feature branch. Not full production or 72h soak.

## P15 HAOS backup importer sanitizer fuzz — 26 October 2026 [ACTIVE]
Owner SentinelX LXC700, branch test/p15-haos-archive-fuzz-20261026, base 2828751. Existing 4 fuzzed surfaces lack new P8aa/P8be full HAOS unprotected backup tar→registry+opaque YAML parser. Add fifth real libFuzzer target on actual importer, only fabricated archives, bounded inputs, strong anti-panic invariant, official sanitizer CI and independent corpus contract. Never rerun old tests as new proof, no household data/VM100. Release remains 0/15, security gate partial. Rollback revert branch.
