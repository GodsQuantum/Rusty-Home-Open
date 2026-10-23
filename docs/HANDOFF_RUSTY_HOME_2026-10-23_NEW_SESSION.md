# Rusty Home — Handoff opérationnel vers nouvelle session

**Date : 23 octobre 2026.** Projet : Rusty Home, produit domotique universel à moteur natif Rust, compatible avec l’écosystème Home Assistant, pas une reproduction d’une installation personnelle.

## Source et accès

- Dépôt public assaini : https://github.com/GodsQuantum/Rusty-Home-Open
- Sources de développement : Cloud9, LXC700, dossier **/srv/storage/nvme/DEV/Projets/Applications/Rusty-Home-Open**.
- Accès distant **exclusivement via SentinelX**. Pour cargo/gh, utiliser la LXC700 sous l’utilisateur arezki via `sudo -n /usr/sbin/pct exec 700 -- runuser -u arezki -- bash -lc '<commande>'`. Ne pas utiliser Remote Desktop Commander.
- Ancien miroir `/srv/storage/nvme/DEV/Projets/Applications/HA-Rust-Bridge` : conserve quatre modifications existantes (AGENTS.md, HANDOFF_CURRENT.md, README.md, STATUS.md) ; **ne pas réinitialiser**.
- Historique privé original de Rusty-Home : ne pas rendre public, réécrire ou supprimer. Le dépôt Rusty-Home-Open est un clone public nettoyé.
- LXC700 seule zone DEV ; ne jamais modifier HAOS VM100, données personnelles, radios réelles, conteneurs de production, secrets, backups, montages réseau ou configuration Cloud9 en production sans consentement spécifique.
- La tâche automatique « Rusty Home — développement continu » est configurée pour une reprise **horaire** (fréquence maximale). Il faut néanmoins avancer substantiellement à chaque exécution. Attention aux sessions concurrentes : **vérifier git/PR en premier et ne jamais écraser leur travail**.

## État exact constaté au moment du handoff

- **main public : `6aa947d`**, PR #31 fusionnée (Ingress Supervisor authentifié en lecture seule). Toutes les modifications de #24 à #31 sont sur main.
- **PR #32 : OUVERTE** — https://github.com/GodsQuantum/Rusty-Home-Open/pull/32 — SHA `02a91b3a69afffc63da466b553f6cf411b809c19`. Auteur d’une session concurrente ; observation réelle des opérations : GET authentifié des jobs puis GET du nouvel état des Apps, rapprochement des postconditions ; **aucune mutation**. Neuf contrôles GitHub vérifiés au vert dans cette reprise. À réexaminer puis fusionner sans violer les protections ; l’état peut changer.
- **PR #33 : OUVERTE** — https://github.com/GodsQuantum/Rusty-Home-Open/pull/33 — SHA courant `097c67d057695a58809796b30da616a736d2b142`, **deux commits**, dont correction des URL front-end derrière le préfixe officiel Ingress. Onze contrôles GitHub vérifiés au vert (y compris App Docker réelle, HAOS 18.3 KVM et navigateur synthétique). Ne pas se fier seulement au SHA initial `68ba92f`. À réexaminer puis fusionner après prise en compte de PR #32 (ou dans l’ordre évitant les conflits), en revalidant la CI de chaque HEAD et les protections.
- Un worktree de documentation **séparé** a été créé : `/srv/storage/nvme/DEV/Projets/Applications/Rusty-Home-Handoff` sur la branche `docs/rusty-home-handoff-20261023`, base main `6aa947d`. Ne pas confondre avec le worktree PR #33.
- Aucun passage sur HAOS de production ni certification du produit final ; **Rusty Home n’est pas terminée**.

## Fonctionnalités réellement ajoutées récemment

- PR #24 : catalogue Supervisor universel, recherche, filtres, pagination, suppression des heuristiques de noms de marques et de l’ancien compteur « 9 Apps ».
- PR #25 : GET des détails de n’importe quelle App via routes officielles V1/V2, typage et expurgation des options/confidentiels.
- PR #26 : navigateur avec recherche par nom, source, pagination 24 cartes et tests sur 83 Apps arbitraires.
- PR #27 : GET des Apps installées `/addons` ou `/v2/apps`, différences installation/suppression/état/version, zéro liste fixe.
- PR #28 : GET de jobs `/jobs/{uuid}` ou `/v2/jobs/{uuid}`, suivi progression/échec/sous-tâches, 100 % ≠ succès.
- PR #29 : réconciliation du résultat job ET de la postcondition réelle (installer, démarrer, arrêter, redémarrer, mettre à jour). Toujours lecture seule.
- PR #30 : lecteur d’Apps installées **branché sur le backend Rust/UI**, option `--supervisor-live`, token officiel injecté, échec explicite si indisponible.
- PR #31 : mode Ingress officiel pour lecture seule avec contrôle de la source réseau, identité injectée par Supervisor et rejet de fausses identités. Un test sur environnement réel HAOS reste nécessaire ; les vérifications HTTP factices ne le remplacent pas.
- PR #32 ouverte : met en chaîne client GET job + GET snapshot pour observer une action (voir tests `live_operation_observation.rs`).
- PR #33 ouverte : vrai packaging **expérimental** `packaging/product-app/rusty_home/` avec `config.yaml`, `Dockerfile` scratch, Rust musl non-root, `scripts/test_product_app_container.sh`, CI dédiée `.github/workflows/product-supervisor-app.yml`, correction du préfixe Ingress des requêtes web. Conteneur Docker démarré localement en LXC700, faux administrateur bloqué par HTTP 403. Ce n’est pas une publication de production ni la preuve du fonctionnement avec de vraies Apps et radios.

## Architecture et décision de compatibilité

- **Produit universel** : toute intégration et toute App compatible avec les API officielles, y compris futurs fournisseurs tiers. Les comptes « 29 intégrations », « neuf Apps », les fixtures d’inventaire et « 0/15 release gates » **NE constituent PAS une liste de compatibilité** et ne mesurent pas le développement produit. Ils ne servent qu’aux régressions historiques.
- Core 2026.10.x : HACS garde ses responsabilités actuelles. Core 2026.11-dev : Marketplace officiel en cours d’intégration ; ne pas développer un second HACS. Supervisor Apps ≠ Marketplace intégrations. Pour l’architecture, lire `docs/MARKETPLACE_CORE_2026_11_ARCHITECTURE.md` et `docs/ROADMAP.md`. Vérifier l’upstream lorsqu’un changement de version entre en jeu.
- Pour les intégrations Python : utiliser les API officielles et un chemin de compatibilité Core/Python, pas prétendre à une exécution native Rust universelle.
- « Native first » et pas de wrappers superflus. Pas de filtre par marque/vendor. L’application doit fonctionner pour d’autres personnes sans leur infrastructure particulière.

## Prochain travail : FINIR LE PRODUIT (pas certification)

1. **Évaluer/fusionner PR #32 et #33** seulement après revérification des checks au HEAD exact et d’éventuels conflits ; privilégier validation indépendante GitHub, ne pas affaiblir les protections.
2. **Rendre l’App expérimentale réellement utilisable dans HAOS jetable**, pas VM100 : installer depuis un dépôt de test isolé uniquement avec les fichiers propres à `packaging/product-app/rusty_home`, vérifier démarrage, Ingress via le véritable proxy Supervisor, URL prefix, permissions d’accès, user ID/roles réels, nomadisme des versions, logs, redémarrage et images/architectures. Ne pas ajouter un `repository.yaml` racine qui publierait accidentellement les App fixtures du laboratoire.
3. **Créer identité et RBAC de confiance** : Home Assistant/Supervisor natif, sujet utilisateur vérifié côté serveur, rôle administrateur vérifié via Core, protection CSRF/replay, confirmation explicite et audit persistant. Les structures `managed_apps` et `rbac_lab` sont encore **feature-gated LAB**, donc ne pas considérer leur booléen de consentement comme une authentification.
4. **Implémenter réellement les actions génériques Supervisor** install/start/stop/restart/update (ainsi que sauvegarde/restauration), en respectant les routes officielles, déclenchement asynchrone, suivi de job réel et postconditions déjà développées ; jamais des requêtes forgées depuis le navigateur. Tester en environnement jetable avec de multiples Apps de tiers, y compris échecs/coupures; empêcher commandes doubles, dérives de version et actions non autorisées. Les opérations de restauration doivent vérifier le backup avant toute écriture.
5. **Frontend fonctionnel** : détails d’Apps en direct, actions conditionnées à la permission serveur, progression/échec, reconnexion, erreurs; moteur natif, automatismes, entités, intégrations universelles, installation, mises à jour, sauvegardes et récupération. Tout le plan de structure doit être achevé avant la certification finale.
6. Seulement **APRÈS complétion fonctionnelle réelle**, mener une campagne distincte de certification globale et produire la version production-ready ; ne pas annoncer un pourcentage de certification d’un produit incomplet.

## Commandes et références pour reprise

Depuis LXC700 dans le dépôt :
```bash
git fetch origin
git status --short --branch
git log -3 --oneline origin/main
gh pr list -R GodsQuantum/Rusty-Home-Open --state open
gh pr checks 32 -R GodsQuantum/Rusty-Home-Open
gh pr checks 33 -R GodsQuantum/Rusty-Home-Open
cargo fmt --all -- --check
cargo test --locked --all --quiet
cargo clippy --locked --workspace --all-targets -- -D warnings
python3 scripts/test_public_redaction.py
bash scripts/test_product_app_container.sh
```

Les deux derniers tests demandent le bon environnement (musl + Docker pour le conteneur). Ne pas exécuter les scénarios HAOS sur la VM100.

Sources de vérité : Issues [#9](https://github.com/GodsQuantum/Rusty-Home-Open/issues/9), [#11](https://github.com/GodsQuantum/Rusty-Home-Open/issues/11), [#12](https://github.com/GodsQuantum/Rusty-Home-Open/issues/12), code et PR GitHub. La CI ne vaut que pour le commit exactement vérifié.

## Règles de livraison exigées par l’utilisateur

- Ne plus renvoyer l’utilisateur à « continue ». À chaque session, **coder, tester, ouvrir/fusionner les PR admissibles et avancer la vraie fonctionnalité suivante**, avec un compte rendu minimal et factuel.
- Ne pas arrêter la progression après six minutes uniquement parce qu’une PR est fusionnée ; la tâche automatique ne se déclenche qu’au maximum toutes les heures, mais une session active peut continuer immédiatement.
- Ne pas certifier par morceaux la future application complète ; tests de développement normaux et CI sont nécessaires, certification finale formelle vient après.
- Si blocage outils/CI/autorisation : l’indiquer, ne rien inventer et ne pas contourner les restrictions.
- Aucun déploiement ou changement de machine de production implicite.
