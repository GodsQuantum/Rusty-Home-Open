# Rusty Home — handoff permanent

**Audit du 24 octobre 2026.** La référence historique de base est `main` 6585cb4 ; elle ne représente pas nécessairement le dernier commit. Pour connaître le HEAD du dossier et les critères actuels : `python3 scripts/show_handoff.py`.

## État confirmé

Produit **NOT_PRODUCTION_READY** : 15 critères, **9 partiels, 6 bloqués, 0 totalement certifiés**. Noyau Rust natif encore synthétique et connexion Home Assistant principalement en lecture seule.

**NE PAS répéter comme nouveaux tests :** sauvegardes et restaurations HAOS complètes/chiffrées, refus du mauvais mot de passe, SHA-256 File Editor, rollback App, installation synchrone Supervisor et récupération HAOS→deuxième VM HAOS vierge (6/6). Lire le [registre](CERTIFICATION_EVIDENCE_REGISTER.md) avant toute tâche.

## Les sources de vérité

- [Audit exhaustif des 15 critères, des alternatives et des délais](PRODUCTION_CERTIFICATION_MASTERPLAN_2026-10-24.md) et [données structurées](../config/certification-masterplan-audit-20261024.json).
- [Preuves de laboratoire](CERTIFICATION_EVIDENCE_REGISTER.md) et [provenances JSON](../config/certification-evidence-registry.json).
- [Journal des travaux](WORK_LOG.md) et [gates de production](../config/release-gates.json).

## Reprise obligatoire

1. Lire ces sources, vérifier le SHA actuel de GitHub, les PR ouvertes, les preuves et la CI avant de relancer des travaux.
2. Utiliser une branche et un worktree DEV isolés ; **ne pas modifier Home Assistant familial**.
3. Prioriser un moteur Rust + OCI et le vrai Core/HACS comme voie de compatibilité généraliste. Tester la migration HAOS→runtime Rust ; ne pas refaire HAOS→HAOS.
4. Faire progresser authentification réelle/permissions, restauration Rust et rollback, Apps/driver radio physiques, endurance 72 h, publication signée, audit indépendant.
5. Dans chaque PR substantielle, actualiser **WORK_LOG.md, ce handoff et les preuves pertinentes** ; vérifier tous les contrôles GitHub sur le SHA final avant fusion.

**Cadence observée au 24 octobre :** 76 PR fusionnées (8 le 22, 50 le 23, 18 le 24 octobre UTC) ; médiane création→fusion 9,0 minutes. Ce nombre ne traduit pas une certification de production. L’objectif universel interdit toute liste blanche limitée aux 9 Apps ou 29 intégrations d’essai.
