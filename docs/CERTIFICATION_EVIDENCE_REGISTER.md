# Rusty Home — registre canonique des preuves et anti-redondance

**Mise à jour : 24 octobre 2026.** Ce fichier est la référence pour éviter de refaire inutilement les essais. La source structurée vérifiée par CI est [`config/certification-evidence-registry.json`](../config/certification-evidence-registry.json). Toute nouvelle validation doit y être inscrite **avec le test, l'environnement, le commit exact, le run GitHub et les limites**, puis ajoutée au [journal du projet](WORK_LOG.md). Ne jamais remplacer une preuve existante par une estimation ni confondre test de laboratoire et gate de production.

## Réponse à la question « la restauration a-t-elle déjà été testée ? »

**Oui, plusieurs fois sur du véritable HAOS 18.3**, en QEMU/KVM isolé. [Campagne prouvée : 17/17 tests GitHub, zéro erreur](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40127707580), commit exact `8dde4a49315ee009c1c45341d5730f17668ea8f3`. Il ne faut **pas** refaire ces scénarios en les présentant comme encore absents.

| Identifiant stable | Scénario déjà couvert | Résultat / portée |
| --- | --- | --- |
| `haos-full-backup` | Archive complète Supervisor, SHA-256 | Vérifié en QEMU |
| `haos-full-restore-same-guest` | Données `share` corrompues puis restauration complète, identité Core | Vérifié **dans la même VM** |
| `haos-encrypted-full-backup-restore` | Backup complet protégé, données altérées puis restaurées | Vérifié **dans la même VM** |
| `haos-wrong-password` | Mot de passe erroné : refus sans mutation des données | Vérifié en QEMU |
| `haos-real-file-editor-partial-data` | File Editor réel : restauration du contenu avec SHA-256 | Vérifié en QEMU |
| `haos-versioned-app-rollback` | App locale V1→V2, mutation, manager SIGKILL, retour V1 et SHA-256 | Vérifié en QEMU |
| `haos-sync-install-release` | Installation native synchrone, inventaire officiel puis déverrouillage ACID | Vérifié en QEMU |
| `haos-async-expired-job-fence` | Job terminé et effacé : aucune reprise dangereuse de POST | Vérifié, **fail-closed seulement** |

Les scripts sources sont `scripts/p8c2_real_haos_supervisor_pytest.py`, leur sélection CI `.github/workflows/p8c2-haos-qemu.yml` et le contrôle strict `scripts/check_p8d_junit.py`. **Le compteur 17/17 désigne tous les scénarios sélectionnés, pas huit tests de restauration ni 17 intégrations certifiées.**

L'ancienne campagne [du 22 octobre 2026](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39873207970) avait déjà validé 16/16 ; la campagne plus récente 17/17 est la référence.

## Nouvelle preuve : restauration sur DEUX VM HAOS différentes

**Réussie le 24 octobre 2026.** La campagne [GitHub 40135221183](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40135221183), SHA exact 8d3b1263f0cb65f11fdbb9f3979a58af052b3731, a réussi **3/3 tests source + 3/3 tests cible**, sans saut. Elle est séparée des 17/17 tests SAME-VM ci-dessus.

La VM source produit une vraie archive complète Supervisor, l'envoie à un relais local borné et enregistre son SHA-256 et boot ID. La deuxième VM part du disque officiel HAOS 18.3 intact, doit avoir un boot ID différent et aucun fichier marqueur source ; elle télécharge le tar, vérifie la taille et le SHA-256, importe puis restaure la sauvegarde avec la vraie CLI HAOS, et démontre la récupération des octets exacts et de Home Assistant Core/Supervisor. Voir [protocole et limitations](CROSS_VM_DISASTER_RECOVERY_2026-10-24.md).

**Ce qui reste NON prouvé :** la migration d'un foyer HAOS vers le moteur **Rusty Home**, les bases de toutes les applications tierces, le remplacement de Core et du Supervisor, les contrôleurs physiques, comptes réels, réseau et rollback d'un matériel entier. La seconde VM prouve HAOS→HAOS, pas HAOS→Rusty Home.

## Protocole obligatoire pour toute nouvelle session

1. Lire **ce registre**, [le journal](WORK_LOG.md), [les gates](CERTIFICATION_LEDGER.md), `scripts/check_p8d_junit.py`, puis contrôler les statuts et commits GitHub actuels.
2. Pour chaque tâche proposée, chercher le cas par identifiant stable, vérifier sa couverture et son *périmètre* ; préférer améliorer la lacune distincte plutôt que réexécuter la même preuve.
3. Utiliser une branche et un worktree isolés dans `/home/arezki/Projets/Applications/` (LXC 700). Aucun essai destructif sur le foyer. Conserver l'état `NOT_PRODUCTION_READY`.
4. Modifier le registre JSON **dans la même PR** que les preuves, mettre à jour ce Markdown et inscrire date, sha, PR, tests et risques dans `WORK_LOG.md`. Le script `scripts/test_certification_evidence_registry.py` refuse une preuve dont le nom n'est plus compilé dans la campagne HAOS.
5. Attendre les CI GitHub **du SHA exact**, fusionner après tous les checks requis verts et écrire l'issue dans le journal. Ne déclarer un gate de production certifié qu'avec la procédure de qualification indépendante dédiée.

**Production : NOT_PRODUCTION_READY, 0/15 gates certifiés.** Un test réussi ne transforme pas automatiquement un gate en certification de production.
