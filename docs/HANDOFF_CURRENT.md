# Rusty Home — handoff permanent

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

## Reprise obligatoire

1. Lire ces sources, vérifier le SHA actuel de GitHub, les PR ouvertes, les preuves et la CI avant de relancer des travaux.
2. Utiliser une branche et un worktree DEV isolés ; **ne pas modifier Home Assistant familial**.
3. Prioriser un moteur Rust + OCI et le vrai Core/HACS comme voie de compatibilité généraliste. Tester la migration HAOS→runtime Rust ; ne pas refaire HAOS→HAOS.
4. Faire progresser authentification réelle/permissions, restauration Rust et rollback, Apps/driver radio physiques, endurance 72 h, publication signée, audit indépendant.
5. Dans chaque PR substantielle, actualiser **WORK_LOG.md, ce handoff et les preuves pertinentes** ; vérifier tous les contrôles GitHub sur le SHA final avant fusion.

**Cadence observée au 24 octobre :** 76 PR fusionnées (8 le 22, 50 le 23, 18 le 24 octobre UTC) ; médiane création→fusion 9,0 minutes. Ce nombre ne traduit pas une certification de production. L’objectif universel interdit toute liste blanche limitée aux 9 Apps ou 29 intégrations d’essai.
