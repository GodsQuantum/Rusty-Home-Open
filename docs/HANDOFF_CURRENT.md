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

## Reprise obligatoire

1. Lire ces sources, vérifier le SHA actuel de GitHub, les PR ouvertes, les preuves et la CI avant de relancer des travaux.
2. Utiliser une branche et un worktree DEV isolés ; **ne pas modifier Home Assistant familial**.
3. Prioriser un moteur Rust + OCI et le vrai Core/HACS comme voie de compatibilité généraliste. Tester la migration HAOS→runtime Rust ; ne pas refaire HAOS→HAOS.
4. Faire progresser authentification réelle/permissions, restauration Rust et rollback, Apps/driver radio physiques, endurance 72 h, publication signée, audit indépendant.
5. Dans chaque PR substantielle, actualiser **WORK_LOG.md, ce handoff et les preuves pertinentes** ; vérifier tous les contrôles GitHub sur le SHA final avant fusion.

**Cadence observée au 24 octobre :** 76 PR fusionnées (8 le 22, 50 le 23, 18 le 24 octobre UTC) ; médiane création→fusion 9,0 minutes. Ce nombre ne traduit pas une certification de production. L’objectif universel interdit toute liste blanche limitée aux 9 Apps ou 29 intégrations d’essai.
