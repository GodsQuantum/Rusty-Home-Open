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
`{"real_host_transport", "actual_user_revocation", "connection_recovery", "privilege_separation"}`

### A. Transport d'écriture hôte authentifié (`real_host_transport`)
- **Existant :**
  - Dans `crates/supervisor-apps/src/official_transport.rs`, `preview_execute()` est strictement gardé par `HARS_P8H_QEMU_ONLY=1` et refuse toute exécution en build par défaut (`ManagedPreviewDisabled`).
  - `core_corroborated_permit_lab.rs` et `core_user_rbac_lab.rs` sont des oracles de décision hors-ligne ou de prévisualisation en lecture seule (`preview_core_authorization`).
- **Écart :**
  - Absence d'un transport d'écriture hôte réel joignant une requête d'écriture HTTP/REST/WS entrante avec émission de jeton de session / CSRF et validation par-requête du périmètre (`RequestScope`).
  - Le système ne dispose pas encore d'un gestionnaire de transport sécurisé pour les mutations d'état host->Rusty validant le non-rejeu temporel et l'intégrité des opérations.

### B. Identités Core et Révocation réelle (`actual_user_revocation`)
- **Existant :**
  - `core_user_snapshot_lab.rs` analyse des trames WebSocket pré-enregistrées.
  - `core_user_live_lab.rs` sait interroger `config/auth/list` en WebSocket en lecture seule sous QEMU jetable.
- **Écart :**
  - Il n'y a pas de synchronisation dynamique ni de révocation immédiate (invalidation de session active à la volée dès qu'un compte Core passe de `ActiveAdmin` à `Denied` ou est supprimé).
  - Aucune preuve de déconnexion/rejet immédiat d'une session en cours d'un utilisateur révoqué sur le canal d'écriture hôte.

### C. Séparation des privilèges et déni multi-utilisateurs (`privilege_separation` & cross-user denial)
- **Existant :**
  - `rbac_lab.rs` applique des rôles (`Administrator`, `AppOperator`, `Household`, `ReadOnly`) sur des requêtes préparées (`PreparedRequest`).
  - `persistent_session_lab.rs` stocke des sessions dans redb (0600) avec confirmation unique par jeton opaque.
- **Écart :**
  - Pas de matrice d'isolation croisée prouvant qu'un utilisateur `Household` ou `AppOperator` ne peut ni usurper, ni intercepter, ni rejouer une confirmation d'un administrateur (`cross-user denial`).
  - Absence de protection CSRF / session hijacking stricte liant le jeton de session et le token CSRF à l'identité physique de l'appelant.

### D. Reprise sur incident / Perte de connexion (`connection_recovery`)
- **Existant :**
  - P8ei a validé la panne 503 sur le mock go-auth MQTT.
  - `durable_dispatch_fence_lab.rs` implémente un verrou atomique dans redb pour empêcher le double POST après crash/SIGKILL.
- **Écart :**
  - Pas de validation d'endurance prouvant la reconnexion convergente avec reprise d'état consistante après coupure du canal hôte-Rust.

---

## 3. Contraintes d'environnement et de gouvernance CT400/CT700

1. **CT400 (Paperclip runner) :**
   - Environnement d'exécution sandbox sans compilateur `cargo` / `rustc` installé.
   - Les tests Python, scripts de conformité de licence et release gate verifiers (`check_release_gate.py`, `test_release_gate_attestation.py`, `test_release_gate_evidence_truth.py`) s'exécutent avec succès en local.
2. **CT700 (dev-build NVMe) :**
   - Environnement disposant de `cargo`, `rustc`, Docker et des cibles de compilation statique musl.
   - Le profil `ssh-actions` sur CT700 est configuré avec une allowlist en lecture seule stricte (`read-command`), refusant l'exécution arbitraire de commandes interactives ou d'écriture (`safe` tool filter).
3. **GitHub / CI :**
   - Aucun jeton copié ou privilège élevé non contrôlé ; les PRs doivent être soumises et vérifiées via la chaîne GitHub Actions officielle.
   - 0/15 release gates certifiées pour la production ; interdiction stricte de déclarer une conformité de production à partir d'un labo.

---

## 4. Recommandation pour les prochaines étapes de CLO-218

1. **Définir et intégrer le module de contrat de transport d'écriture sécurisé :**
   - Implémenter dans le modèle de crates (ou sous forme de composant de laboratoire dédié avec manifestes audités) la validation des scopes par-requête, la vérification CSRF / session et le rejet croisé inter-utilisateurs.
2. **Adjoindre la suite de tests de contrat Python/Rust :**
   - Valider formellement la non-régression sur `scripts/test_firstparty_license_manifests.py` et le maintien strict de `check_release_gate.py --expect-blocked`.
3. **Déléguer les compilations lourdes à Codex / CT700 ou à la CI GitHub :**
   - Soumettre une PR minimale documentée et testée pour validation par la CI GitHub indépendante avec revue contradictoire (Codex / Claude) avant intégration.

---

## 5. Livrable atomique CLO-229 (P1a · Denial cross-user RBAC reproductible en Core synthétique)

1. **Test négatif multi-utilisateurs déterministe implémenté :**
   - Fonction de test `synthetic_multiuser_rbac_cross_user_and_revocation_denial` ajoutée dans `crates/supervisor-apps/src/core_corroborated_permit_lab.rs`.
   - **User A** écrit sur sa propre fixture : compte Core actif admin + grant sur `app_user_a` -> confirmation et permis accordés avec succès (`permit.is_some()`).
   - **User B** refusé sur la ressource de User A : `issue_checked_fixture_confirmation` refuse (`None`), `grant_checked_fixture_status` refuse (`None`) et tentative de détournement de jeton de session / claim de User A rejetée fail-closed (`None`).
   - **User A expiré / révoqué** refusé : session expirée (`now > ttl`), compte Core révoqué (`CoreAccountStatus::Denied`) et révocation explicite de session dans redb (`sessions.revoke`) refusent immédiatement toute émission de permis.
   - **Aucun effet de bord hôte et auditabilité stricte :** journal d'audit `audit.jsonl` vérifié, contenant les enregistrements chaînés par hash cryptographique SHA-256 pour chaque décision (refus loggés), sans aucune mutation ni processus système déclenché.
2. **Garde-fous de contrat Python et CI :**
   - `scripts/test_p8i_synthetic_core_multiuser_rbac.py` : vérifie statiquement et unitairement le respect du contrat multi-utilisateurs et le maintien des verrous contre tout transport privilégié par défaut.
   - `scripts/test_core_corroborated_preview_contract.py` : assertions enrichies pour garantir la présence des tests négatifs multi-utilisateurs.
   - `.github/workflows/ci.yml` : étape CI intégrant le test de contrat Python et le test Rust dédié.
3. **Statut des release gates :**
   - 0/15 certifiées. `python3 scripts/check_release_gate.py --expect-blocked` vérifié conforme (`NOT_PRODUCTION_READY`). Aucune porte de production n'est déclarée validée sans preuve physique réelle Core indépendante.
