# Rusty Home — audit exhaustif, accélérations et plan de certification

**État vérifié : 24 octobre 2026.** Base Git : [main au commit 6585cb4](https://github.com/GodsQuantum/Rusty-Home-Open/commit/6585cb41d95dc3a04c52309d2ad8f3405064dd7f). Source machine : [audit JSON](../config/certification-masterplan-audit-20261024.json), calculé à partir des **15 vraies entrées** de [release-gates.json](../config/release-gates.json). Consulter impérativement le [registre de preuves](CERTIFICATION_EVIDENCE_REGISTER.md) et le [journal des actions](WORK_LOG.md) avant de proposer de répéter un test.

## Verdict sur les oublis

**Il existait des preuves réelles non reconnues dans certains anciens textes de blocage.** La restauration complète dans HAOS, sa variante chiffrée, la mauvaise clé refusée, la restauration exacte de File Editor, le rollback d'une App V1/V2 et la restauration complète HAOS→seconde HAOS neuve sont DÉJÀ validés en vraie VM QEMU. Références : [17/17 sur même VM](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40127707580) ; [3/3 source et 3/3 cible sur deux VM](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40135221183). La certification ne doit pas refaire ces tests comme s'ils étaient nouveaux.

Le classement actuel est **9 partiels + 6 bloqués = 15 critères, 0 intégralement certifié**. Des preuves de laboratoire ne deviennent pas une certification du produit complet sans essais sur la véritable application et critères indépendants. La preuve HAOS→HAOS n'est PAS la preuve HAOS→Rusty Home.

L'audit du code confirme : crates/native-kernel est encore décrit comme noyau synthétique, crates/ha-compat comme adaptation WebSocket de laboratoire en lecture seule, crates/supervisor-apps comme inspection Supervisor principalement read-only avec mutations réservées au laboratoire. Les plus gros manques ne sont PAS un nouveau test de sauvegarde, mais la compatibilité opérationnelle.

## 15 gates — acquis, manque, effort restant

L'effort est estimé en **reprises de travail substantiel avec code, tests et CI**, pas en simples messages « continue ». Les plages se recouvrent fortement et ne se somment pas : aucune vitesse GitHub ne peut raccourcir un soak de 72 h ni une revue de sécurité humaine.

| Gate officiel | Statut | Acquis à ne PAS refaire | Preuve restante | Reprises ciblées* |
| --- | --- | --- | --- | --- |
| authenticated_host_transport | Partiel | Ingress réel ; faux en-têtes rejetés ; UID non-root. | End-to-end authenticated host→Rust write transport; per-request scopes, revocation, cross-user denial, actual QEMU+production-like threat review. | 2–5 |
| ha_core_hacs_29_runtime | Partiel | Core 2026.10.2 réel ; plusieurs HACS réels en CI. | Core 2026.10.3 compatibility baseline; 29 representative integration lifecycles in CI (not a 29-integration product limit), upgrade/downgrade, config flow, automations, restarts. | 6–12 |
| hacs_github_oauth | Partiel | Intégrations HACS chargées ; OAuth interactif non validé. | Authorize live OAuth flow with token renewal, expiry, revocation, errors, simultaneous users; CI secret isolation. | 2–5 |
| apps_9_lifecycle | Bloqué | Store générique ; installation synchrone officielle et suivi. | Use 9 representative REAL third-party Apps as conformance fixtures; verify generic install/start/update/restart/stop/backup/rollback/cleanup including DB state. | 4–10 |
| supervisor_apps_restore | Partiel | File Editor réel restauré octet pour octet et versionnement local. | Persist and restore multiple stateful App DBs beyond File Editor; verify backup schema and partial, failed and encrypted restores. | 3–7 |
| haos_backup_restore | Partiel | Restauration complète/chiffrée ; seconde VM neuve : 6/6. | Already proven same-VM + second HAOS VM. New gap is import/transform restored HAOS entities, automations, registries, secrets into Rusty Home runtime with fidelity and reversibility. | 3–8 |
| rollback_verified | Bloqué | Retour V2→V1 d'une App après SIGKILL prouvé. | Already proven one real versioned App rollback; missing complete failed-host Rust cutover and return to functioning original with accounts/DB/device IDs unchanged. | 3–7 |
| single_radio_owner | Bloqué | Verrouillage multi-processus et crash fencing simulés. | Real Zigbee/Thread/Z-Wave radio ownership transfer/failover with hardware adapters, exclusive leases, no dual commands. | 3–7 |
| ha_ws_reconnection | Partiel | Lecteur WS et tests bornés avec Core. | 24-72h websocket loss/recovery, backpressure and state convergence tests on official Core event streams under realistic load. | 2–5 |
| production_ui_auth | Bloqué | Validation d'identité Core et consentement en labo. | Production multi-user write RBAC, CSRF protection, sessions, admin revocation and replay-denied audited mutations. | 3–7 |
| native_drivers_services | Bloqué | Moli, MQTT 5 et pilotes synthétiques ; pas de parité matérielle. | Prove vendor-neutral real-driver coverage and service automation beyond synthetic Rust hub. For true Pure Rust Core parity this is open-ended. | 12–35 |
| performance_full_stack | Bloqué | Microbenchmarks et quelques charges isolées. | Measure end-to-end entity throughput, WS latency, restart latency, CPU/RAM vs HAOS, and 72h soak plus faults. | 3–7 |
| packaging_updates | Partiel | Install, downgrade, purge et APT signé en labo. | Reproducible multi-arch signed release artifacts and rollback on existing staging install, cold boot, upgrade and uninstall. | 3–7 |
| third_party_licenses | Partiel | SBOM SPDX 217 dépendances et attestation de source. | SBOM/source already signed; independently review source+binaries+assets+OCI image redistribution and signed release obligations. | 2–6 |
| security_stress_fuzz | Partiel | RustSec et quatre surfaces fuzzées/ASan. | Threat model and independent penetration test of genuine writable routes, longer fuzz corpora, incident/rollback and remediation retests. | 3–8 |

\* Ces fourchettes sont des hypothèses de planification non contractuelles ; elles ne sont **pas** des échéances, ni des probabilités de succès. Des ressources matérielles et des tiers peuvent modifier l'ordre et la durée.

## Deux objectifs TRÈS différents

**Voie A — Rusty Home comme App / façade sécurisée au-dessus de HAOS :** l'authentification, le Supervisor et Home Assistant Core officiels restent présents. Utile pour une première exploitation parallèle mais **ne remplace ni HAOS ni Core**. Hypothèse : 6–15 reprises substantielles, 2–5 semaines selon les contrôles de sécurité et l'endurance.

**Voie B — remplacer HAOS/Supervisor avec un gestionnaire Rust, garder le vrai Core/HACS :** installer Core dans un conteneur, orchestrer les applications tierces en OCI et reprendre les fonctions de sauvegarde, de réseau, de permissions et de mises à jour. C'est la voie la plus rapide pour une **parité d'usage généraliste** sans réécrire l'immense bibliothèque Python. Mais le Supervisor officiel hors HAOS est NON SUPPORTÉ ; un nouvel orchestrateur Rust doit porter les garanties manquantes. Hypothèse indicative : 30–70 reprises substantielles et 2–5 mois ou davantage, **si** matériel disponible, flux secrets/permissions prouvés et audit indépendant possible.

**Voie C — remplacer également Home Assistant Core par un moteur intégralement Rust :** le noyau actuel ne dispose que de pilotes synthétiques et de fonctions MQTT expérimentales. Les nombreuses intégrations, entités, workflows, automatismes, OAuth, API, Matter/Thread et pilotes multi-fournisseurs doivent être portés ou exécutés par une couche de compatibilité Python. Une API de lecture seule n'est pas une compatibilité exécutive. Hypothèse très incertaine : **100–300+ reprises majeures et 12–36+ mois**, potentiellement plus pour la parité générique, avec changements continus de l'écosystème. Il serait trompeur de donner une date précise.

L'architecture suggérée est donc **noyau Rust + OCI + MQTT 5 + Core/HACS existants comme moteur de compatibilité optionnel**, puis portage progressif des domaines vers Rust. Le produit peut rester universel ; la matrice « 29 intégrations » et « 9 Apps » sont seulement **échantillons de certification**, jamais une liste blanche de fonctionnalités.

## Ordre concret de développement sans redondance

1. **Inventaire automatique, contrats de compatibilité et matrice de preuves.** Exécuter une fois les fixtures officielles à chaque changement d'API Core/Supervisor ; comparer l'état et les registres. Ne refaire ni backup HAOS même VM ni seconde VM si les fichiers correspondants ne changent pas.
2. **Stratégie d'orchestration et données.** Choisir et formaliser la voie B comme chemin court, sans nier l'objectif ultime C. Couvrir Core/Apps en OCI génériques, gestion d'état, crash/restart, version pinning et sauvegardes; migrer les informations d'un backup HAOS **dans le runtime Rust**. Comparer les enregistrements et exécuter un rollback complet.
3. **Identité / sécurité d'écriture.** Garder l'Ingress officiel pour la voie A ; pour l'hôte autonome, revocation par utilisateur, session, anti-CSRF, privilèges minimaux, journal d'opérations, POST durable exactement une fois et traitement des jobs disparus. Tester attaques de concurrence et redémarrages.
4. **Compatibilité et matériel.** Tester 29 intégrations représentatives sur le Core officiel (et 2026.10.3), puis matrix HACS réelles et applications conteneurisées. Ajouter Zigbee2MQTT, Z-Wave JS, Matter.js et MQTT sans refaire les piles radio, avant d'écrire des drivers Rust spécialisés. Certifier propriété exclusive des radios physiques et absence de double commande.
5. **Reliabilité et exploitation.** WebSocket reconnexion et resynchronisation, scénarios de pertes réseau, 72 h de soak réel avec charge, observabilité et budgets de performance mesurés par rapport à HAOS, récupération d'une mise à jour ratée.
6. **Publication et indépendance.** Construire images OCI précompilées multi-architecture signées, SBOM de source et image, contrat de mise à jour/réversion, revue de licence et audit externe de sécurité. Exiger les 15 preuves de production indépendantes avant d'autoriser le basculement.

Les étapes peuvent se développer en parallèle par groupes, mais leurs validations de cutover sont séquentielles : identité et données avant écriture, rollback complet avant passage en production, endurance et audit externe avant publication générale.

## Raccourcis validés par la documentation officielle au 24 octobre 2026

- [Installation officielle](https://www.home-assistant.io/installation/) et [types de déploiement](https://contributors.home-assistant.io/os/installation-types/) : seuls HAOS et Container sont supportés ; les anciens « Supervised » et « Core Python venv » ne sont plus des chemins de production supportés. L'exécution du Supervisor officiel sur un hôte Linux arbitraire est déclarée non supportée : **ne pas baser notre architecture sur cette supposition**.
- [Home Assistant 2026.10 changelog](https://www.home-assistant.io/changelogs/core-2026.10/) : version corrective **2026.10.3 datée du 23 octobre**, plus récente que le pin 2026.10.2 du labo. Ajouter une matrice 2026.10.3 et 2026.11-dev en tests non destructifs ; NE PAS remplacer silencieusement l'ancienne version témoin.
- [Règles de qualité, mises à jour le 12 octobre](https://developers.home-assistant.io/docs/core/integration-quality-scale/rules/) et [méthode de test Core](https://developers.home-assistant.io/docs/development_testing/) : 54 règles, pytest, couverture, fixtures, hassfest. **Réutiliser les suites officielles pour certifier les adaptateurs**, plutôt que créer un nombre arbitraire de scripts synthétiques.
- [Apps et OCI](https://developers.home-assistant.io/docs/apps) et [publication](https://developers.home-assistant.io/docs/apps/publishing/) : les Apps sont des conteneurs ; des images multi-architecture précompilées via CI évitent les compilations lentes chez l'utilisateur.
- [Sécurité Ingress](https://developers.home-assistant.io/docs/apps/presentation/) : identité déléguée à HA, isolation 172.30.32.2 et pénalités des privilèges manager/full_access ; réduire les privilèges et les chemins d'auth plutôt qu'en créer.
- [MQTT 5 et découverte](https://www.home-assistant.io/integrations/mqtt/) : discovery standard, métadonnées d'identité et retained/LWT. Le crate Rust MQTT existe déjà ; cela fournit une voie de compatibilité dynamique pour plusieurs vendeurs.
- [Matter HA](https://www.home-assistant.io/integrations/matter/) : le Matter controller est un serveur séparé avec WebSocket. **Attention nouveauté 2026 :** le [Matter Server officiel v9 a été migré de Python vers matter.js](https://github.com/home-assistant/apps/blob/master/matter_server/CHANGELOG.md); le [python-matter-server est abandonné](https://github.com/matter-js/python-matter-server/blob/main/README.md). Brancher la pile matter.js actuelle évite d'implémenter ou de maintenir un backend périmé.
- [Zigbee2MQTT](https://github.com/zigbee2mqtt/hassio-zigbee2mqtt) et [Z-Wave JS UI MQTT](https://github.com/zwave-js/zwave-js-ui/blob/master/docs/guide/mqtt.md) : réutiliser ces contrôleurs et leur modèle de découverte/commande, tout en ne transférant le propriétaire USB/radio qu'après tests d'exclusivité.

**Ne PAS chercher à raccourcir les tests qui prouvent le risque réel :** restauration après échec, sécurité mutante, radio physique, endurance et revue humaine. Le raccourci autorisé est la **réutilisation de briques éprouvées**, pas de qualifier des essais synthétiques comme de la production.

## Chronologie réelle des interactions et vitesse observée

Le dépôt public analysé commence le 22 octobre. Il existait des prototypes et échanges auparavant ; ce premier commit public n'est **pas** la date réelle de début du projet.

| Fenêtre UTC | PR fusionnées | Résultat démontré |
| --- | ---: | --- |
| 22 octobre, 22:53–23:53 | 8 | Démonstrations Rust/CI, premier vrai HAOS KVM, premières intégrations HACS et packaging |
| 23 octobre, 00:14–23:56 | 50 | Store universel en lecture, dashboard, Ingress, attestations SBOM, Core identité, suivi des jobs, davantage de sécurité |
| 24 octobre, 00:21–21:49 | 18 | Verrous de POST durables et SIGKILL, exécution synchrone officielle, backups renforcés, seconde VM vierge 6/6, registre anti-redondance |

**76 PR fusionnées**, du 22 octobre 22:53 au 24 octobre 21:49 UTC. D'après les métadonnées GitHub : médiane d'une PR entre création et fusion **9,0 minutes**, 75e percentile **28,1 minutes**. Ces durées comprennent du travail en parallèle et des corrections CI ; elles ne sont ni du temps de développement actif ni une garantie pour les tâches restantes. Malgré 76 merges en 47 h, **0 gate complet** a changé de statut. Il serait fallacieux de prévoir 15 gates/76×47h ou « quelques continue ».

Pour les interactions de la période du 21 au 24 octobre, les points de reprise montrent un passage des premières campagnes HAOS 12/12 et 13/13 à 16/16, 17/17 puis 3/3+3/3 sur deux VM. Les longues séries de « continue » ont donc produit de nombreux **jalons de laboratoire** rapides, mais peu de **grandes couches fonctionnelles de parité Core**. Une reprise de travail approfondie peut donner plusieurs petites PR documentaires en quelques heures ; un cycle complexe d'automatisation, de migration ou de radio physique peut consommer plusieurs reprises et des jours de test.

**Timeline de travail, non engagement de livraison :** App auxiliaire contrôlée 2–5 semaines ; gestionnaire Rust préservant Core 2–5+ mois ; remplacement complet autonome de Core 12–36+ mois, très incertain. Le meilleur moyen d'accélérer de façon responsable est de réduire le code à écrire grâce aux composants existants tout en maintenant le périmètre généraliste.

## Règle du handoff

À chaque session : consulter ce plan + le registre + le journal ; inspecter main et les PR GitHub ; choisir un gap NON couvert ; modifier les preuves et le journal dans la même PR ; attendre la CI du dernier SHA exact ; fusionner et mettre à jour le handoff. Voir [handoff actuel](HANDOFF_CURRENT.md) et le contrat de cohérence CI. **Ne jamais faire une qualification de production à partir d'un compteur de PR ou d'un simple PASS CI synthétique.**
