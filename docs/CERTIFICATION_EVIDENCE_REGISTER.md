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

## Preuves nouvelles — Core officiel vers Rust/Moli (#89–#95)

Ces sept résultats indépendants sont désormais consignés dans le [registre de migration native structuré](../config/native-migration-evidence.json). Les run GitHub cités ont été vérifiés en `success` au **SHA source exact** le 25 octobre 2026, et les sept PR sont fusionnées. Les scénarios sont différents de la restauration HAOS→HAOS documentée ci-dessus.

| Identifiant stable | Preuve et portée maximale | Source vérifiée |
| --- | --- | --- |
| `core-registry-stage` | #89 : véritable Core 2026.10.2, identités vers redb Rust et rollback, sans états ni action | [GitHub 40140787860](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40140787860) |
| `core-registry-real-moli-hub` | #90 : vrais identifiants Core dans le véritable Hub Moli, lecture seule | [GitHub 40141864751](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40141864751) |
| `core-real-state-snapshot` | #91 : états synthétiques générés par le vrai Core vers Moli, instantané borné | [GitHub 40142758906](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40142758906) |
| `loopback-event-convergence` | #92 : événements WebSocket fictifs authentifiés vers Moli, déconnexion et révocation | [GitHub 40144794037](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40144794037) |
| `bounded-ws-moli-runner` | #93 : binaire Rust invocable, WebSocket loopback fictif, sortie bornée | [GitHub 40145513687](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40145513687) |
| `official-core-ws-to-moli` | #94 : authentification et vrais événements WebSocket Core officiel→Moli sur réseau Docker inexistant, avec rollback | [GitHub 40211747111](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40211747111) |
| `official-core-ws-moli-consistency` | #95 : huit événements Core, refus de mauvais token, cohérence des états et révocation fail-closed | [GitHub 40213008548](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40213008548) |

La présence d'un chemin Core→WebSocket→Rust→Moli **ne valide PAS** le remplacement de Home Assistant Core, les services ou automatisations, les contrôleurs radios, la migration des données de foyer, les intégrations tierces au complet, une reconnexion durable ou une restauration HAOS→Rust→HAOS. Ce sont des preuves de laboratoire bornées avec de vraies images Core ; le compteur reste **NOT_PRODUCTION_READY — 0/15**.

## P8aa — véritable archive Supervisor HAOS vers identités Rust certifiée en laboratoire

Le nouveau Rust lit un tar full Supervisor non chiffré et importe uniquement core.entity_registry vers redb, avec rollback. CI prévue : vrai Core 2026.10.2 dans un tar synthétique et, séparément, vrai Supervisor HAOS 18.3 en VM jetable produisant un tar avec identités fictives, importées en Rust. La source, le test et leurs limites sont consignés dans docs/HAOS_TO_NATIVE_BACKUP_STAGE_2026-10-25.md. Résultat VÉRIFIÉ : run https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40238122942, SHA PR #99 9c6e3220f433153c0dc28ef0d2549bd52a345a8f, merge d297bbbd29ac313723b54794ad27aceaf0636166 ; 3/3 tests sélectionnés dans HAOS 18.3 QEMU, archive officielle full non chiffrée, import du registre (2 identités fictives) dans redb, activation CAS et rollback. Preuve durable dans config/haos-native-backup-evidence.json. Ceci ne prouve ni restauration complète Core, ni Apps, ni automatisations, ni secrets. Les sept preuves Core→Rust historiques et HAOS→HAOS restent acquises, production 0/15.

## Protocole obligatoire pour toute nouvelle session

1. Lire **ce registre**, [le journal](WORK_LOG.md), [les gates](CERTIFICATION_LEDGER.md), `scripts/check_p8d_junit.py`, puis contrôler les statuts et commits GitHub actuels.
2. Pour chaque tâche proposée, chercher le cas par identifiant stable, vérifier sa couverture et son *périmètre* ; préférer améliorer la lacune distincte plutôt que réexécuter la même preuve.
3. Utiliser une branche et un worktree isolés dans `/home/arezki/Projets/Applications/` (LXC 700). Aucun essai destructif sur le foyer. Conserver l'état `NOT_PRODUCTION_READY`.
4. Modifier le registre JSON **dans la même PR** que les preuves, mettre à jour ce Markdown et inscrire date, sha, PR, tests et risques dans `WORK_LOG.md`. Le script `scripts/test_certification_evidence_registry.py` refuse une preuve dont le nom n'est plus compilé dans la campagne HAOS.
5. Attendre les CI GitHub **du SHA exact**, fusionner après tous les checks requis verts et écrire l'issue dans le journal. Ne déclarer un gate de production certifié qu'avec la procédure de qualification indépendante dédiée.

**Production : NOT_PRODUCTION_READY, 0/15 gates certifiés.** Un test réussi ne transforme pas automatiquement un gate en certification de production.

## P8ab — Native Rust archive custody and genuine second HAOS guest

New distinct proof (not old direct HAOS->HAOS): PR #101 at 58cbfe3023532b7acca68123641a58692fab541d, merged 4757377d5d773844f6e278af4712597d50cd799a, GitHub https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40240344194 SUCCESS. Both independent genuine HAOS 18.3 QEMU source 3/3 and target 3/3 pass, with opaque byte-exact private native Rust escrow in the middle, file replaced by native Rust reexport and verified before target restore. This does not qualify actual host takeover or real Apps/radios; production 0/15. Machine evidence in config/native-backup-escrow-evidence.json.

## Native Rust guest-host evidence P8ac–P8af — independently green genuine HAOS KVM (26 October)

P8ac PR #103, official HAOS serial boot via Rust process, GitHub 40243098816 (18/18); P8ad PR #104, official Supervisor AND Core CLI health, GitHub 40243728521 (11/11); P8ae PR #105, official HAOS COW persisted across QEMU SIGKILL restart, GitHub 40247000475 (12/12); P8af PR #106, second independent Rust manager reattached with exclusive flock and rejected concurrent writer, GitHub 40247834775 (13/13). All merged into main d8c5bee817c7b7e8ef5ea6af24d62d366fbe610a. Exact approved HEAD SHA and workflow proof markers in config/haos-rust-guest-evidence.json. Real 18.3 QEMU lab, fictional data only; no family access. They prove usable native Rust host primitives, not a distributable host daemon, secure operational App service, radio, hardware handover, full reverse migration or 72-hour reliability. Production 0/15.

## Immutable verified P8ag–P8al Rust host evidence (26 Oct 2026)

- P8ag PR #108: 14/14 exact feature-head GitHub CI, run 40250349459; long-lived Rust/QMP lifecycle.
- P8ah PR #109: 15/15 exact feature-head GitHub CI, run 40251693506; fixed 127.0.0.1 UI forwarding.
- P8ai PR #110: 16/16 exact feature-head GitHub CI, run 40252930987; real official Core HTTP frontend.
- P8aj PR #111: 10/10 exact feature-head GitHub CI, run 40254861417; real systemd two HAOS/QEMU process lifecycles.
- P8ak PR #112: 11/11 exact feature-head GitHub CI, run 40255870443; reproducible Debian guest package real dpkg lifecycle.
- P8al PR #113: 9/9 exact feature-head GitHub CI, run 40257162323; offline signed APT index and tamper refusal.

All six are MERGED, latest main a756696b60081d641ad7ec9cf43144f3361a201b. Do NOT rerun as newly unproved work. Machine-readable source config/haos-rust-guest-evidence.json keeps each exact original feature SHA, merge SHA and workflow marker source. P8ag–P8aj official HAOS 18.3 QEMU/KVM is an isolated genuine guest; P8ak real Ubuntu dpkg and P8al cryptographic offline APT use disposable CI runners. No household deployment, no signed production trust key, no actual App service parity, no physical radios, no whole-host backwards cutover or endurance qualification. Production remains **0/15** fully certified.
