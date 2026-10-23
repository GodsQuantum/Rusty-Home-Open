# Rusty Home — plan de certification vérifiable

**Révision : 23 octobre 2026.** Objectif : **15/15** critères de production,
séparément d'un véritable feu vert de publication.
Le Marketplace de Home Assistant Core doit rester la source officielle
des intégrations ; Rusty Home ne doit pas maintenir un clone incompatible.

Une campagne GitHub réussie en environnement isolé est une preuve technique
partielle, pas la certification globale. Chaque critère exige un artefact de
validation immuable, son SHA-256, une vérification indépendante et une
approbation de mise en production distincte.

| Critère | Priorité | Critères objectifs manquants |
| --- | --- | --- |
| authenticated_host_transport | P0 | Reboot and revoke live Supervisor user; test forged ingress, expired credential, outage and per-request least privilege across HAOS and Docker |
| ha_core_hacs_29_runtime | P0 | Run all 29 integrations through create/configure/update/restart/recover/remove with real domain fixtures; demonstrate Marketplace parity |
| hacs_github_oauth | P1 | Interactive GitHub sign-in, refresh, revoke, duplicate users, offline recovery and device-code expiration tested |
| apps_9_lifecycle | P0 | Nine actual third-party Supervisor Apps each install/start/update/stop/uninstall with persistent state checks and no app-specific hacks |
| supervisor_apps_restore | P0 | Real arbitrary App configuration, data, ingress and service state survive backup and restore with identical post-restore behavior |
| haos_backup_restore | P0 | Backup real user-equivalent data, destroy/reinstall guest, restore encrypted archive and verify all relevant data and services |
| rollback_verified | P0 | Fail migration after partial commit and restore previous instance/state; verify identical data and no external device contention |
| single_radio_owner | P0 | Real Zigbee/Thread/Matter radio owner transfer and crash/USB fault tests confirm exactly one controller at every transition |
| ha_ws_reconnection | P1 | 24h reconnection soak across Wi-Fi loss, token expiry, WS shutdown, back-pressure and restart with convergent state |
| production_ui_auth | P0 | Live Supervisor user sessions with CSRF/origin, per-App RBAC, revocation, one-time confirmation and non-replay verified |
| native_drivers_services | P1 | Real USB/serial/BT/radios/services and firmware interoperability matrix on HAOS, Docker and VM |
| performance_full_stack | P0 | Representative complete 24h-72h soak, resource budgets, failures, stable state and measurable speed vs baseline |
| packaging_updates | P0 | Production-signed Linux artifacts and Home Assistant images upgrade and rollback against preexisting installations without data loss |
| third_party_licenses | P0 | SPDX SBOM signed, all third-party licenses and source/NOTICE obligations reviewed with six unresolved sources resolved |
| security_stress_fuzz | P0 | Full attack-surface long-run fuzz, unauthorized HTTP tests, privilege escalation review, pen test and audited remediation |

## Prochains jalons exécutables

1. Authentification complète de la session HAOS native, confirmation anti-rejeu et audit durable pour une App de laboratoire.
2. Restauration intégrale d'une véritable App tierce jetable, avec comparaison des volumes et vérification du fonctionnement après rollback.
3. Valider les installations antérieures aux mises à jour signées ; construire la chaîne de publication et révocation des clés.
4. Démontrer les migrations Core et les 29 intégrations depuis une source Marketplace officielle.
5. Tester l'unicité des contrôleurs radio et le comportement en panne matérielle avant toute migration domestique.
6. Produire pour chaque gate une preuve machine, audit indépendant et validation de publication.

## Garde-fous

- Ne modifier aucune instance Home Assistant domestique ou VM de production pour les essais.
- Pas de migration irréversible sans sauvegarde vérifiée et restauration démontrée.
- Ne jamais réduire le périmètre d'un critère pour pouvoir l'annoncer certifié.
- Les 13 résultats GH Actions documentés sont des **preuves de laboratoire**.
- Le compteur de critères de production reste **0/15** tant qu'aucun ne satisfait les exigences ci-dessus.
