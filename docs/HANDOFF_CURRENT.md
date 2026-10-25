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

New P8ab worktree Rusty-Home-P8ab-20261025 from origin/main e7235f7. See crates/interop-moli/src/backup_escrow.rs, scripts/test_p8ab_backup_escrow.py and .github/workflows/p8ab-native-escrow-rollback.yml. The distinct P8ab test inserts a native, private, durable Rust archive vault between the already proven two official HAOS QEMU guest boots. Crucial: must await the exact P8ab CI success, no production 15 gate signoff, no actual Core or App execution. Preserve all earlier QEMU, Core and Moli proof IDs.

## Reprise obligatoire

1. Lire ces sources, vérifier le SHA actuel de GitHub, les PR ouvertes, les preuves et la CI avant de relancer des travaux.
2. Utiliser une branche et un worktree DEV isolés ; **ne pas modifier Home Assistant familial**.
3. Prioriser un moteur Rust + OCI et le vrai Core/HACS comme voie de compatibilité généraliste. Tester la migration HAOS→runtime Rust ; ne pas refaire HAOS→HAOS.
4. Faire progresser authentification réelle/permissions, restauration Rust et rollback, Apps/driver radio physiques, endurance 72 h, publication signée, audit indépendant.
5. Dans chaque PR substantielle, actualiser **WORK_LOG.md, ce handoff et les preuves pertinentes** ; vérifier tous les contrôles GitHub sur le SHA final avant fusion.

**Cadence observée au 24 octobre :** 76 PR fusionnées (8 le 22, 50 le 23, 18 le 24 octobre UTC) ; médiane création→fusion 9,0 minutes. Ce nombre ne traduit pas une certification de production. L’objectif universel interdit toute liste blanche limitée aux 9 Apps ou 29 intégrations d’essai.
