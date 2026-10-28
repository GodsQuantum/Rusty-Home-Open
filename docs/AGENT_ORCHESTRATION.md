# Orchestration Paperclip multi-CLI — Rusty Home

**Référence technique 2026-10-28.** Exécution : control plane Paperclip CT400, build lourd CT700/GitHub CI, source NVMe CT700. Ne pas créer de services custom si l'upstream Paperclip sait déjà gérer issues, workspaces, queues, skills, budgets, approvals, routines et sessions persistantes.

## 1. Mission et rôles existants (pas de nouveaux agents par défaut)
- CTO Cloud9 (Hermes) : TRIAGE / architecte, délègue une issue bornée par propriétaire ; décide du meilleur agent selon coût, contexte et complexité, préserve feuille de route ; fait replanifier les échecs plutôt que recommencer.
- Codex : Rust/Go/code exigeant, modifications ciblées ; pas de déploiement ; Git worktree d'issue.
- Claude : conception / revue complexe indépendante et tests adversariaux ; **ne jamais auto-approuver son propre commit**.
- Gemini : recherche et comparaison de documentation/docs/tests légers ; s'abstenir de grosses analyses répétées.
- OpenCode : corrections rapides bornées dans l'issue, intégrations et scripts.
- Pi : patchs étroits, triage de logs et quick checks.
- CEO Cloud9 : arbitrage uniquement si blocage CTO et décision stratégique ; pas de seconde exécution parallèle automatique du même ticket.
- Dify spécialistes : seulement pour un champ nécessitant spécialité ; aucune activation généralisée. Conserver les routines existantes et quotas associés.

## 2. Couche Paperclip : règles opérationnelles
- Une société Cloud9, projet Rusty Home dédié, goal hiérarchique, issues P0/P1/P2, dépendances explicites, assignee unique. Les commentaires de ticket contiennent SHA, CI, liens, diagnostic, prochain objectif concret.
- Le CTO choisit l'assignee ; issue checkout atomique. En cas de 409 conflit de lock, ne pas retry agressivement. Conserver une session par ticket ; ne pas redémarrer un CLI à chaque étape.
- Workspaces Paperclip en mode *git_worktree* isolé, branché sur main, branche Paperclip associée à une issue, sans overlap ; ne pas diriger les agents vers le miroir nocturne HDD. Si workspace principal CT400 distinct de CT700, seul le repo GitHub synchronise l'éphémère ; aucun build lourd en CT400. Projets futurs : choisir une seule racine canonique CT700 et un workspace géré Paperclip qui n'est jamais un miroir.
- CI : ciblée locale en premier, en CI exacte du commit ensuite, tests larges UNE FOIS après fonctionnalité, pas de relances quand aucune modification. PR seulement après checks, revue indépendante par agent distinct, merge soumis à contrôle CI, sans bypass.
- Démarrage d'issue : charger 1 handoff court + AGENTS.md + statut machine ; rechercher les preuves par identifiants. Ne pas ingérer les 180k de HANDOFF_CURRENT.md ni les logs entiers en prompt.
- En cas de blocage : identifier racine/risque, réparer automatiquement les cas sans impact ; ticket BLOCKED motivé s'il faut matériel, identité réelle ou décision d'utilisateur ; poursuivre le prochain ticket indépendant. Éviter les boucles de wake.

## 3. Politique budgets / tokens
- Flash/minimal pour classer issues, lire métadonnées, cibler grep et logs CI ; performant pour hypothèses techniques/tests ; modèle de revue fort seulement pour changement de sécurité, schéma, migration.
- Aucun quorum obligatoire d'agents : 1 auteur + 1 relecteur indépendant pour code risqué ; utiliser 1 seul agent pour tâche triviale ; comparaison multi-agent seulement si décisions difficiles et non redondantes.
- Borne par ticket : chemins, critères acceptation, nombre max de tentatives sur une même erreur, stop si coûts/quotas et remplacement par solution locale si justifié. Jamais tourner une boucle infinie pour contourner un service externe.
- Concurrence CTO = 1 ; max 1-2 contributeurs actifs sur branches différentes après workspace validé. Pas 20 exécutions simultanées par CLI.
- Ne pas inscrire de secrets dans instructions, issues ou logs. Le coût Paperclip ne capture pas nécessairement tous les abonnements/prix ; ne pas extrapoler 'gratuit' en absence de métriques.

## 4. Architecture et états
Source canonique : CT700 /home/arezki/Projets/Applications/Rusty-Home-Open = Cloud9 /srv/storage/nvme/DEV/Projets/Applications/Rusty-Home-Open.
Paperclip live : CT400 /srv/lxc/ia-studio/appdata/paperclip/runtime, base DB sous .../data, API privée URL privée Paperclip configurée dans CT400 (port 3110), jamais publiée dans Git.
Git remote : https://github.com/GodsQuantum/Rusty-Home-Open.git ; main = seule version intégrée.
Miroir HDD : /srv/storage/cloud/arezki/Projets : sauvegarde SyncBridge 02:00, INTERDIT de développer directement ici.
Paperclip workspace : CT400 data/instances/default/projects/<company>/... ; clone éphémère séparé du source canon et nettoyé après clôture ; ne pas diriger les agents vers le clone sans credentials git fonctionnels et tests de branche réussis.
Documents courts : PAPERCLIP_HANDOFF.md + AGENTS.md + STATUS.md. Historique long : docs/HANDOFF_CURRENT.md, uniquement sur demande/ticket particulier.

## 5. Terminologie et critères « fini »
- TEST LAB : CI verte sur fixtures jetables, valeur de lab-evidence.status=verified_lab_only ; PAS une gate.
- GATE : preuve indépendante et complète pour critères de config/release-gates.json, signée si exigé.
- READY : 15/15 certifiés, audités, performance et rollback réels éprouvés. Avant : NOT_PRODUCTION_READY, deployment_authorized=false.
- Ne pas changer statut par modification textuelle sans preuves. Documenter les vraies limites et la provenance de chaque assertion.
- Les 69 claims/65 CI du 28/10 sont une baseline vérifiée ; ne pas donner de faux pourcentage global. Une vraie tâche réduisant une gate vaut davantage que 20 micro-tests répétitifs.

## 6. Boucle autonome mesurable
1. CTO se réveille sur assignation ou routine Paperclip ; confirme la liste des tâches et ouvre le handoff court.
2. Vérifie si déjà fait via GitHub, SHA, evidence registry et tests. Actualise issue sans recompiler inutilement.
3. Choisit le CLI et crée des tâches de taille PR, assignee et dépendances ; imposer mini contrat de tests et rollback.
4. Agent exécute, note commandes essentielles/CI, ouvre PR. Relecteur distinct vérifie sans présumer le succès.
5. Après green GitHub exact SHA, merge et synchronisation, enregistrer preuve LAB et re-run release checker ; fermer issue, fermer workspace si close-readiness OK.
6. CTO choisit le prochain blocage et programme la suite automatiquement tant que le budget le permet. Escalade humaine seulement pour secrets/accès réellement indisponibles, matériel réel ou actions irréversibles.
7. Tous projets futurs suivent la même arborescence et un skill Paperclip partagé ; ne pas copier ces longs prompts dans chaque sous-agent, les référencer.

## 7. Règles sûreté
Pas de live HAOS VM100, de modification de VM HAProxy/passerelle, de radio physique sans accord. Aucune exécution de shell arbitraire d'un fichier externe avant vérification. Tout processus orphelin ou worktree appartenant à un autre agent est préservé. Ne pas supprimer worktrees actifs juste pour « clean » : inventorier statut Git / PR / lock / PID / timestamp, puis archiver ou supprimer uniquement ceux dont la sûreté est démontrée. Aucun symlink.
