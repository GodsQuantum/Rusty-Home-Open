# Rusty Home — entrée rapide pour Paperclip / toute CLI

Dernière validation source : 28 octobre 2026. Statut : **NOT_PRODUCTION_READY**.
Projet canonique : CT700 /home/arezki/Projets/Applications/Rusty-Home-Open.
Dépôt GitHub : https://github.com/GodsQuantum/Rusty-Home-Open (branche main).
Orchestrateur Paperclip existant : CT400 / API privée port 3110, société Cloud9.
Cette fiche est la porte d'entrée COURTE ; ne pas injecter les 180 ko du journal historique dans chaque prompt.

## Contrat d'exécution
1. D'abord lire AGENTS.md, STATUS.md, README.md et ce fichier. Lire docs/PRODUCTION_CERTIFICATION_MASTERPLAN_2026-10-24.md pour choisir un manque technique réel. Pour les preuves existantes : docs/CERTIFICATION_EVIDENCE_REGISTER.md et config/lab-evidence.json par recherche ciblée, pas lecture exhaustive à chaque run.
2. Les 15 critères production sont obligatoires : 0/15 certifiés au dernier audit ; 69 preuves de laboratoire / 65 CI réussies distinctes. Réexécuter scripts/check_release_gate.py --expect-blocked pour l'état présent. NE JAMAIS interpréter une preuve labo comme permission de déploiement.
3. Reprendre par un ticket Paperclip atomique : chercher issue/dépendances et PR existantes, git fetch, main clean, grep de preuve existante. Ne PAS répéter un scénario démontré. Une issue = un livrable/PR = un seul propriétaire de worktree.
4. Spécification -> tests ciblés -> vraie preuve fonctionnelle sur fixtures officielles isolées -> CI exact HEAD verte -> review indépendante -> merge main -> documentation de preuve -> nettoyage worktree. Ne jamais masquer un échec ou synthétiser un PASS.
5. Le commit de source et le reçu documentaire sont distincts si requis. Inscrire dans docs/WORK_LOG.md les vraies observations, liens CI, limites, rollback et les obstacles.
6. Ne pas arrêter, redémarrer, modifier HAOS VM100 ni les LXC de production ; tests fictifs et environnements éphémères seulement sans réseau externe/radio réel. Toute bascule de la maison, secrets réels, dépenses payantes ou donnée destructrice : APPROBATION EXPLICITE, jamais automatisée.
7. CT700 + NVMe est la source canonique. L'espace de Paperclip CT400 est un workspace **éphémère** et isolé dans l'appdata Paperclip ; le miroir HDD /srv/storage/cloud/arezki/Projets est uniquement un BACKUP nocturne, JAMAIS un cwd de développement, jamais source de push.
8. Pas de symlink, pas de copie permanente du projet entre machines, pas de répertoire temporaire à côté du canon ; tout worktree Paperclip est attaché à issue/branche et doit être clos après merge selon vérification close-readiness.
9. Les comptes et budgets des agents sont gérés via Paperclip ; ne jamais copier API keys, .env, refresh tokens dans Git, logs ou prompts. Dédoublonner les tests, rappeler les chemins et commit SHA plutôt que des dumps de fichiers.
10. Après une passe, créer/mettre à jour la prochaine issue dépendante dans Paperclip et la confier à un autre agent si l'actuel n'est pas optimal. Continuer selon heartbeat et budget sans exiger « continue » ; ne demander l'utilisateur que pour une décision irréversible ou ayant un effet réel.

## Travail prioritaire non redondant
- P0 Débloquer une vraie chaîne de développement PR autonome à partir du workspace isolé Paperclip, autorisations GitHub gouvernées, build CI et CT700 ; prouver sur une micro-PR sûre.
- P1 Déployer le **vrai** Core/Supervisor dans un HAOS QEMU jetable et certifier l'auth_api MQTT, sans comptes du foyer (P8ei validait seulement le vrai plugin contre FAUX Supervisor).
- P1 Migration authentique HAOS -> Rusty Home et rollback complet, y compris identités, registres et plusieurs Apps stateful (pas HAOS->HAOS, déjà testé).
- P1 Core 2026.10.3 + HACS 29 intégrations et 9 vraies Apps sur matrice de conformité ; ne pas transformer les chiffres en whitelist produit.
- P1 sessions/utilisateurs/ACL/RBAC, sockets d'écriture authentifiés, résiliation et anti-rejeu.
- P2 convergence WebSocket avec vraies déconnexions prolongées, contrainte 24-72h ; radios à propriétaire unique sur matériel autorisé, benchmarks 72h et review sécurité indépendante ; reproductibilité OCI/signature/licences.

## Documents
- docs/AGENT_ORCHESTRATION.md : répartition CLI, politique de contexte, sûreté, états CI, critères de fin, reprise.
- docs/HANDOFF_CURRENT.md : ancien historique exhaustif, destiné à consultation ciblée uniquement.
- docs/CERTIFICATION_EVIDENCE_REGISTER.md : sources CI exactes.
- config/release-gates.json : vérité machine des 15 critères.
- docs/PRODUCTION_CERTIFICATION_MASTERPLAN_2026-10-24.md : évaluation de l'effort restant (30-70 reprises voie B ; pas un nombre fixe de CI).

## Résumé du travail de cette passe
Plan préalable : créer ce handoff concise, améliorer AGENTS.md et créer docs/AGENT_ORCHESTRATION.md ; faire CI et PR docs, sans toucher aux binaires produits, API live, ni services. Repli : supprimer uniquement la branche de documentation en cas d'échec, conserver le canon et les autres worktrees intacts.
