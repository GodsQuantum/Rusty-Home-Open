# Rusty Home — instructions communes à toutes les CLI et agents Paperclip

# Contributing safety rules

Never connect these tests to a real radio, home automation server, password vault or actual household. Never claim production certification without independent evidence.

Entrée obligatoire COURTE : [PAPERCLIP_HANDOFF.md](PAPERCLIP_HANDOFF.md). Politique détaillée : [docs/AGENT_ORCHESTRATION.md](docs/AGENT_ORCHESTRATION.md).

1. Vérifier branche Git, issue Paperclip, worktree assigné, CI et preuves existantes AVANT de modifier. Une issue = un propriétaire de branche, sans collision.
2. CT700 /home/arezki/Projets/Applications/Rusty-Home-Open est le dépôt canonique NVMe ; GitHub synchronise les worktrees éphémères CT400. Ne jamais développer sur le miroir HDD /srv/storage/cloud/arezki/Projets. Aucun doublon/symlink/service custom.
3. Le projet est NOT_PRODUCTION_READY ; l'ancien registre LAB ne certifie aucun des 15 critères production. Toujours garder VM100 HAOS et services du foyer intacts. Pas de vraies radios, identités, secrets ni déploiement avant approbation.
4. Respecter les documents racines Cloud9 lorsqu'ils sont accessibles (AGENTS.md, PATHS.md, NOUVEAU_PROJET.md). Lorsqu'un CLI n'est pas sur CT700, utiliser la copie Git versionnée et ce handoff sans essayer de parcourir un miroir du répertoire NVMe.
5. Petits tests -> CI fonctionnelle indépendante exact HEAD -> revue par un second agent -> merge -> preuve exacte -> mise à jour handoff -> workspace cleanup. Jamais de faux PASS.
6. Déléguer selon spécialité et budget ; pas de multi-LLM pour tâches triviales. Résumer le diff et les sources, jamais ingérer le gros historique complet ou secrets.
7. En cas de vraie impossibilité, consigner une issue BLOCKED avec conditions de déblocage, puis prendre la prochaine tâche indépendante. Ne réclamer une intervention humaine que pour validation irréversible, credentials ou manipulation physique.
