# Inventaire technique d'écart : Authenticated Host Write Transport (CLO-218)

**Date d'analyse :** 28 octobre 2026
**Branche d'exécution :** `CLO-218-p1-transport-d-criture-s-curis-et-multi-utilisateur`
**Base commit :** `6581d97801554c2d36c54f94d1b31ad74d675a52` (origin/main)
**Gate cible du Masterplan :** `authenticated_host_transport` (0/15 certifiés, statut actuel `lab_partial`)

---

## 1. État des lieux et preuves déjà acquises dans le dépôt

Le registre de preuves officiel (`config/lab-evidence.json`, `docs/CERTIFICATION_EVIDENCE_REGISTER.md`) compte 69 preuves de laboratoire vérifiées pour 65 CI distinctes. Sur la release gate `authenticated_host_transport`, 4 preuves labo sont actuellement enregistrées :
1. `native_rust_security_suite` (CI run 40004128682) : validation des refus de faux pairs / IP spoofing sur l'ingress et RBAC nominal en mémoire.
2. `official_product_nonroot` (CI run 40004128675) : conteneur Superviseur App exécuté en UID non-root avec ingress authentifié.
3. `mqtt_supervisor_delegated_auth_loopback_mock_lab` (PR #255, CI run 40660045891) : mock Rust loopback TCP du protocole d'authentification Superviseur /auth.
4. `ha_mosquitto_official_goauth_mqtt_revoke_ci` (PR #257 / #258, CI run 40664390147) : validation du plugin officiel Mosquitto `go-auth.so` avec révocation de session sans cache et fail-closed sur panne 503.

---

## 2. Inventaire exact des écarts techniques (Gaps)

Pour satisfaire le contrat de release et les 4 scénarios obligatoires de `scripts/release_attestation.py` pour `authenticated_host_transport` :
`{"real_host_transport", "actual_user_revocation", "connection_recovery", "no_privileged_fallback"}` :

### Gap 1 : Absence de transport d'écriture authentifié par requête vers l'hôte réel
- **Constat :** Les seuls transports d'écriture existants sont le mock MQTT TCP loopback (`crates/supervisor-apps/src/official_transport.rs` sous feature `managed-apps-preview`) et des mocks d'exécution de conteneur.
- **Invariants inviolables :**
  - La compilation standard (`--all`) interdit tout transport d'écriture réel sans feature explicite (`no_privileged_fallback`).
  - Aucun transport hôte non authentifié ni élévation implicite sans confirmation à usage unique (consentement durable one-shot).
  - L'attestation de release bloque immédiatement si une preuve tente d'utiliser une fixture synthétique pour certifier la gate finale en production.

### Gap 2 : Vérification multi-utilisateur et déni cross-utilisateur
- **Constat :** Les tests actuels valident l'administrateur unique ou le rejet d'IP forgée, mais n'avaient pas encore figé sous forme de test négatif automatisé la séparation stricte de privilèges entre plusieurs utilisateurs Core actifs où un utilisateur B tente d'écrire sur la ressource de l'utilisateur A.
- **Résolution dans CLO-229 (P1a) :** Implémentation du test négatif synthétique déterministe dans `crates/supervisor-apps/src/core_corroborated_permit_lab.rs` couvrant :
  1. User A autorisé sur sa fixture.
  2. User B rejeté (confirmation impossible, statut refusé) sur la ressource de User A.
  3. Rejet immédiat de toute tentative de session expirée, compte Core révoqué, ou session révoquée.
  4. Auditabilité stricte sans effet de bord hôte (chaîne de hachage `audit.jsonl`).

### Gap 3 : Absence d'environnement matériel / VM100 en laboratoire CI public
- **Constat :** La machine cible de production (HAOS VM100) est explicitement hors périmètre de test pour les agents autonomes. La CI publique GitHub s'exécute sur `ubuntu-24.04`.
- **Règle :** Ne jamais tenter de contourner ou simuler une certification réelle sur machine de production depuis un environnement synthétique. Conserver la gate `authenticated_host_transport` dans son état honnête (`lab_partial`, non certifiée pour la production).

---

## 3. Jalons sûrs pour combler les écarts

1. **Jalon P1a (CLO-229) [Réalisé] :**
   - Test négatif multi-utilisateurs synthétique déterministe et audit trail immuable.
   - Contrat de garde Python `scripts/test_p8i_synthetic_core_multiuser_rbac.py`.
   - CI publique mise à jour et vérifiée verte.
2. **Jalon P1b (Prochain jalon sûr) :**
   - Élaboration d'un harnais QEMU/mock supervisé avec deux instances Core distinctes simulant la révocation dynamique en cours de transport sans toucher aux services réels du foyer.
