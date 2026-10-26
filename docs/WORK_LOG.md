# Rusty Home — journal de développement et de certification

## 2026-10-26 — P8ah native Rust HAOS frontend loopback forwarding (CI pending)

Rust guest host now optionally exposes only QEMU localhost 127.0.0.1:18180 forwarding to official HAOS guest TCP/80, with a real host TCP listener probe. The default lab mode still exposes no ports; no public binding, HA Core token or radio. Genuine KVM CI required before certification; login UI health and production are not yet proven.


## 2026-10-26 — P8ag continuous Rust-managed HAOS guest (CI pending)

New Rust guest host process lifecycle, existing COW disk, private QMP, bounded shutdown and QEMU reap. Exact official HAOS KVM CI pending. No installation to the family system, no radio ownership and no production gate certified.


## 2026-10-26 — pinned results of P8ac–P8af (all green and merged)

- Four non-duplicative official pinned HAOS 18.3 KVM proofs: Rust QEMU boot (P8ac, 18/18), actual Supervisor and Core CLI health (P8ad, 11/11), persistence of COW data across QEMU hard restart (P8ae, 12/12), and an entirely new Rust process reattaching the old guest storage while rejecting concurrent ownership (P8af, 13/13).
- Exact runs 40243098816, 40243728521, 40247000475 and 40247834775; PRs #103–#106 all merged at main d8c5bee817c7b7e8ef5ea6af24d62d366fbe610a. Preserve outcomes and do not retest as new certifications. Exact feature SHA, immutable CI markers and limits in config/haos-rust-guest-evidence.json.
- No changes on the family HAOS/Cloud9 host deployment or actual radios. Full Core API, Apps lifecycle, native Rust service daemon, signed distribution, real radio fencing, 72h soak and independent review still missing. Release 0/15.


## 2026-10-26 — P8af Rust process reattachment (CI pending)

New resume command reopens exact QCOW2 backing source and tests exclusive Rust manager ownership. Independent genuine KVM two-process proof and negative flock test must pass GitHub. Not production.


## 2026-10-25 — P8ae persistent HAOS guest under Rust (CI pending)

- New Rust stateful guest orchestration: atomic/private fixed qcow2 COW overlay from immutable official HAOS backing image, flock exclusive single owner, repeated KVM guests without ephemeral -snapshot, same real official HAOS guest data disk.
- End-to-end new failure scenario: fake HAOS share marker written/synced by serial root in first guest, SIGKILL QEMU process, independent new QEMU process verifies sha256sum of persisted marker. Source image unchanged. Real KVM CI required at exact SHA before proof.
- No real family radio, keys or automations. Core/Supervisor health P8ad remains verified in PR #104 (real HAOS QEMU 40243728521, 11/11 checks); still NOT_PRODUCTION_READY 0/15.


## 2026-10-25 — P8ad real Supervisor/Core response under Rust-owned HAOS VM (CI pending)

- Follow-on from merged P8ac #103 (KVM serial boot), not repetition of its boot test. Native Rust now drives official guest serial console, authenticates local root in disposable HAOS, independently requests official Supervisor and Core CLI info (read-only) and refuses success unless both return result=ok.
- Negative unit cases, structured redacted booleans, bounded retry/timeout and no image mutations. New genuine HAOS 18.3 KVM workflow; verify its exact CI SHA before claiming healthy Core or Supervisor. Persistent guest/App service running and production RBAC/radio ownership are still unproved; 0/15 production.


## 2026-10-25 — P8ac official HAOS guest from native Rust (CI pending)

- New capability: a real Rust process orchestrates official HAOS QEMU KVM boot with pinned image digest, no original image writes, fixed machine args, no radio passthrough, bounded cleanup. Guest still runs original Core/Supervisor/Apps; this is not a native Rust replacement of their functions.
- Isolated GitHub QEMU lab needed at exact source SHA. Detecting the official HAOS login console is an OS boot proof, NOT evidence that Core and Apps are healthy.
- Reuse real HAOS backup/import/rollback P8p-P8ab proofs. Production remains 0/15; no family HAOS touched.


## 2026-10-25 — P8ab, reversible whole-archive escrow between actual HAOS VMs (lab verified)

- New native Rust opaque, content-addressed and fsynced private HAOS archive escrow; never parses secrets, preserves all bytes including encrypted payload, hashes on ingest/export and denies symlinks, corrupt blobs, weak file permissions and overwrite.
- Reuses existing proven P8p QEMU 18.3 source and separate fresh target; adds an actual durable Rust vault in between. The original guest archive is replaced by Rust-exported bytes before second VM restore.
- Local synthetic tests positive and hostile PASS. The genuine two-VM run PASSED: GitHub 40240344194, SHA 58cbfe3023532b7acca68123641a58692fab541d, PR #101 merged at 4757377d5d773844f6e278af4712597d50cd799a; 3/3 first VM, 3/3 second VM; 12 checks PASS.
- Core/Apps/radios DO NOT run in Rust, whole product stays 0/15 production, no family HAOS involved.


## 2026-10-25 — P8aa, vrai tar HAOS vers stockage Rust (laboratoire certifié)

- Anti-redondance : réutiliser les preuves HAOS→HAOS 17/17 et 6/6 et les PR Core→Rust/Moli #89–#98, sans les renommer en certification native.
- Fonction nouvelle : Rust ouvre le tar complet non chiffré, vérifie les en-têtes, le hash, les chemins, les limites, le CRC gzip, récupère seulement core.entity_registry, stocke les identités en redb durable et offre activation CAS / rollback.
- Tests locaux : archives factices valides/hostiles, refus chiffrement, liens et gzip tronqués, aucun service du domicile.
- Test CI distinct : vrai Core 2026.10.2 emballé dans un tar synthétique, plus vraie sauvegarde Supervisor HAOS 18.3 créée dans une VM jetable et importée vers Rust (preuve RÉUSSIE au run 40238122942, SHA 9c6e3220f433153c0dc28ef0d2549bd52a345a8f, merge d297bbbd29ac313723b54794ad27aceaf0636166).
- Dépendances contrôlées : Cargo.lock +9 crates, source SPDX 226 paquets / 522 relations ; pas de clearance légale. Production 0/15.
- Dossier : docs/HAOS_TO_NATIVE_BACKUP_STAGE_2026-10-25.md.


## 2026-10-25 — P8z, sondes Core/WS/Moli et course de livraison

- **Anti-redondance :** PR #97 fusionnée (c8c6a73) après 9/9 CI au SHA feature 141bf49, preuve P8y durable ; anciennes restaurations HAOS→HAOS conservées.
- **Lacune :** sans drainage après get_states, une mise à jour seulement d'attributs peut arriver dans le relais avant le retour du snapshot et entraîner une révocation intempestive.
- **Correction :** lecture en boucle strictement bornée (trois snapshots max, deux intervalles 30 ms), drainage avant/après chaque snapshot, vérification structurale Core/WS/Moli et purge à la troisième divergence. Pas de reconnexion ni de write.
- **Compteur :** les événements consommés pendant la sonde sont comptés dans le runner de laboratoire.
- **Preuves :** nouveau test WS loopback des deux cas livré/perdu et checks de l'espace Rust ; CI exacte à vérifier ; production toujours 0/15.


## 2026-10-25 — P8y, audit du cas attribut-seul WebSocket

- **Réutilisation :** main au commit 2fdcb77, PR #89–#96 déjà intégrées ; preuves de migration et restaurations HAOS conservées.
- **Nouveau bug :** get_states Core peut renvoyer un état textuel identique mais last_updated plus récent si un state_changed d'attribut a été perdu.
- **Correctif laboratoire :** égalité stricte des timestamps ET des attributs JSON officiels Core / miroir WS avant d'autoriser une lecture Moli ; toute divergence révoque le miroir et tous les périphériques.
- **Qualification :** test de régression dédié, CI au SHA exact à vérifier. Connu : déclenchement fail-closed possible pendant une course entre snapshot et livraison WS ; reconnexion/réconciliation durable encore absente. Production 0/15.


## 2026-10-25 — P8x, registre canonique pour sept certifications Core→Rust/Moli sans répétition

- Après fusion des PR #89–#95, preuves exactes vérifiées : commits de tête et commits fusionnés, runs GitHub terminés en succès, marqueurs de tests et limites de chaque preuve.
- Nouveau registre `config/native-migration-evidence.json` pour les étapes Core→Rust (distinct du registre HAOS→HAOS existant), index docs/CERTIFICATION_EVIDENCE_REGISTER.md et contrat strict scripts/test_native_migration_evidence.py intégré dans CI.
- Le contrat rejette les preuves dupliquées, les marqueurs disparus, les liens non conformes et tout glissement de certification de laboratoire en production. Aucune restauration antérieure n'est rejouée.
- La production demeure NOT_PRODUCTION_READY, 0/15 ; les prochains chantiers doivent faire progresser vraie migration HAOS→Rust→HAOS, supervision durable WebSocket, intégrations universelles, services et automatismes.


## 2026-10-25 — P8w, vérification autoritative de la cohérence Core/WS/Moli

- **Anti-redondance :** les PR #89–#94 sont déjà fusionnées et valident l'identité, les états, l'abonnement WS, l'exécutable borné et le vrai Core 2026.10.2 ; les anciennes restaurations HAOS→HAOS restent acquises.
- **Correction :** une réponse get_states réussie ne prouve pas que les événements ont été reçus. Elle est désormais comparée à la ShadowSession et aux états réellement publiés par le Hub Moli. Divergence, horodatage périmé ou état manquant => invalidation globale, suppression des appareils et nouveau snapshot obligatoire.
- **Preuve Core renforcée :** 8 événements successifs attendus, vrai refus auth_invalid d'un jeton fictif erroné. CI GitHub du SHA exact nécessaire avant toute certification du scénario.
- **Tests locaux :** nouveaux tests de divergences et suppression fail-closed, aucune commande physique ; release toujours 0/15.
- **Référence :** docs/HA_CORE_WS_STATE_CONSISTENCY_2026-10-25.md.


## 2026-10-25 — P8v : vrai Core 2026.10.2 + WebSocket authentifié vers Rust/Moli

- **Sans redondance :** PR #89–#91 réelles Core→Moli (registre/états), #92–#93 WebSocket synthétique et runner CLI déjà fusionnées ; restaurations HAOS→HAOS inchangées.
- **Nouveaux scripts :** véritable Core officiel containerisé, compte utilisateur et jeton éphémères, registre et état Core réels, WebSocket Core authentifié ; second conteneur Ubuntu exécute Rust dans l'espace réseau Core – aucun port hôte ni réseau sortant. L'isolement Docker internal avec publication 127.0.0.1 a échoué au test local et a été remplacé par network none + container namespace.
- **CI réelle exigée :** workflow Core dédié + preuve P8V_GENUINE_OFFICIAL_CORE_WEBSOCKET_AUTH_TO_ACTUAL_RUST_MOLI_PASS, export et rollback redb, tests statiques de non-exposition du jeton. Ne pas annoncer de réussite avant CI SHA exact.
- **Limites :** aucun matériel réel, aucune action de service, aucun réseau de foyer, aucun HAOS productif ; 0/15 production. Voir docs/GENUINE_CORE_WS_TO_MOLI_2026-10-25.md.


## 2026-10-25 — P8u : runner WS→Moli utilisable en ligne de commande

- **Réutilise :** PR #89 registre Core, #90 Moli, #91 états Core, #92 événements WebSocket en lecture seule ; restaurations HAOS déjà certifiées inchangées.
- **Nouvelle fonction :** binaire natif avec entrée --lab-ha-ws-to-moli, durée/événements bornés, authentification WebSocket exclusivement loopback, jeton par environnement, accès aux seuls lots Core redb déjà activés, événement réel confirmé par le Hub Moli avant résultat JSON.
- **Tests :** vrai serveur WebSocket fictif authentifié, ordre souscription avant get_states, événement plus récent et roundtrip de vérification, vérification du rejet des commandes et des identifiants non enregistrés. CI et strict Clippy à vérifier au SHA exact.
- **Périmètre :** outil de laboratoire sans service continu, sans droits d'écriture, sans installation familiale ; release 0/15. Documentation : docs/HA_WS_LAB_RUNNER_2026-10-25.md.


## 2026-10-25 — P8t : événements WebSocket authentifiés en laboratoire vers Moli

- Travaux réutilisés : PR #89 registre Core persistant, #90 périphériques Moli, #91 états du Core, miroir et gestion d'époques hass-rs préexistants.
- Nouvelle convergence : changements vers véritable Hub Moli via mpsc borné, acquittement, refus des timestamps anciens et contradictoires, purge unavailable/suppressions, refus des commandes et IDs non enregistrés.
- Test révélateur : fermeture TCP silencieuse non visible immédiatement par hass-rs ; ajout d'une sonde transport get_states bornée qui déclenche la révocation des périphériques.
- Test de réseau authentifié uniquement sur WebSocket loopback fictif. Pas encore de connexion WebSocket productive au vrai Core ou aux équipements de Cloud9. CI exacte exigée pour certifier l'étape.
- Détails : docs/HA_WS_TO_MOLI_EVENT_CONVERGENCE_2026-10-25.md ; statut 0/15 production.


## 2026-10-25 — P8s, instantané des états Core dans vrai Moli OS

- **Déjà acquis, pas répété :** #89 Core→identités Rust + rollback; #90 Core→Moli Hub read-only; sauvegardes HAOS précédentes.
- **Nouvelle tranche :** Core 2026.10.2 écrit réellement deux objets State.as_dict() et Rust consomme l'instantané avec contrôle des identités Core. Le driver Moli ne publie que les états observés, ignore unknown/unavailable, refuse états hors registre et commandes.
- **Sécurité :** fichier privé O_NOFOLLOW et borné; parser strict sans clés en double; tous les points Moli restent non inscriptibles.
- **Preuve réelle :** P8S_REAL_CORE_STATES_OBSERVED_IN_NATIVE_MOLI_READONLY_PASS doit réussir en CI au SHA exact avant toute attestation de l'essai end-to-end.
- **Limites :** aucun abonnement WS continu, aucune automatisation, aucune commande d'appareil, aucune clé privée. Release NOT_PRODUCTION_READY 0/15.
- **Doc :** docs/HA_CORE_STATES_TO_MOLI_SNAPSHOT_2026-10-25.md.


## 2026-10-25 — P8r, entités Core mappées dans le vrai Hub Moli/Rust

- **Pré-existants :** PR #89 export de vrais identifiants Core→redb Rust, et restauration HAOS→HAOS déjà acquise. Aucun replay de ces certifications.
- **Nouvelle fonction :** module native-kernel/ha_registry_shadow, driver Moli sans aucune écriture, points à état inconnu, contrôle anti-collision, vérification de la propriété d'identité et refus réel des commandes Moli.
- **Tests :** 2 entrées puis 1 232 entrées de divers fournisseurs dans un véritable Hub Moli, refus de toute écriture. Validation réelle dans le conteneur Home Assistant Core attendue de la CI GitHub.
- **Prochain manque :** flux des états Core et automatismes vers la machine d'états Moli ; aucune validation production, release reste 0/15.
- **Documentation :** docs/NATIVE_MOLI_HA_REGISTRY_SHADOW_2026-10-25.md.


## 2026-10-25 — P8q, import d'identités HA dans Rust avec rollback durable

- **Anti-redondance :** réutilise la projection HA→Moli existante, et conserve les anciennes restaurations HAOS (17/17 même VM, 6/6 seconde VM) sans les refaire.
- **Travaux :** import strict de core.entity_registry, identités Moli, détection de doublons, validation SHA-256, base redb privée, activation CAS atomique et rollback persistant entre processus. CLI explicitement lab-readonly.
- **Tests locaux :** 338 tests workspace all-features PASS, Clippy strict PASS, 1232 entrées multi-fournisseurs fictives restaurées et mécanismes hostiles refusés.
- **Premier run GitHub de la PR #89 :** échecs transparents de synchronisation du SHA-256 Cargo.lock (SPDX et licences) et appel privé Core Store sans argument. Les preuves ont été régénérées en respectant les contrôles, et le test Core laisse désormais l'enregistrement officiel s'écrire via son timer de 10 secondes. Ne compter le véritable essai qu'après CI au SHA final.
- **Preuve en attente :** véritable Core 2026.10.2 isolé doit produire la sauvegarde .storage puis Rust la lire en CI ; non certifié avant run GitHub vert.
- **Limites :** aucun état dynamique, aucune commande d'appareil, secret ou migration complète HAOS→Rust. Production 0/15.
- **Détails et prochaines tâches :** docs/NATIVE_HA_ENTITY_REGISTRY_STAGE_2026-10-25.md et HANDOFF_CURRENT.md.


## 2026-10-24 — Audit exhaustif des oublis, raccourcis 2026 et rythme réel

- Relecture de origin/main 6585cb41d95dc3a04c52309d2ad8f3405064dd7f, du registre canonique (8 preuves mêmes VM + 1 seconde VM), des 15 gates et des 8 crates Rust : **9 critères partiels, 6 bloqués, 0 intégralement certifiés**.
- PR GitHub vérifiées via API : **76 merges**, 8 le 22/10 UTC, 50 le 23/10, 18 le 24/10, médiane création→fusion **9,0 minutes**. Aucune extrapolation directe à la parité.
- Démasquage du goulot : native-kernel utilise principalement des périphériques synthétiques ; ha-compat lit des données via une session de laboratoire. Les restores réels HAOS vers HAOS ne sont pas un Rusty Home natif prêt.
- Recherche de la documentation officielle d'octobre 2026 : seulement HAOS/Container supportés, version Core 2026.10.3 plus récente que le témoin CI, paquet Matter officiel migré à matter.js ; réutilisation de Core/HACS, OCI, MQTT v5 et piles radio existantes raccourcit le travail.
- Nouveaux documents : docs/PRODUCTION_CERTIFICATION_MASTERPLAN_2026-10-24.md, docs/HANDOFF_CURRENT.md, config/certification-masterplan-audit-20261024.json ; commande scripts/show_handoff.py et test scripts/test_certification_masterplan_contract.py intégrés à la CI, modèle de PR avec mise à jour obligatoire des preuves et du handoff.
- Les trois trajectoires A/B/C et les hypothèses de durée sont dans le masterplan. Aucune autorisation de déploiement ; aucune modification de HAOS familial.


## 2026-10-24 — PR #86 fusionnée, statut source→nouvelle VM clos

- La [PR #86](https://github.com/GodsQuantum/Rusty-Home-Open/pull/86) a réussi tous les contrôles GitHub et a été fusionnée le 24 octobre 2026, à 21:44:37 UTC, dans main : c5e7a82b0e13b521f529d51190868e54e1867751.
- Preuve finale [run 40135221183](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40135221183), SHA exact 8d3b1263f0cb65f11fdbb9f3979a58af052b3731, réussite 3/3 VM source + 3/3 VM cible, aucune omission ; précédent run vert 40134632585.
- Le registre des preuves mentionne désormais la **seconde VM vierge** comme vérifiée. Ne pas refaire ce scénario HAOS→HAOS par erreur.
- Ce n'est PAS la migration vers le moteur Rust et ne valide PAS les comptes réels, radios et bases App tierces. Production toujours NOT_PRODUCTION_READY, 0/15 gates.


## 2026-10-24 — P8p, nouvelle validation inter-VM (3/3 source et 3/3 cible RÉUSSIS en QEMU réel)

- **Anti-redondance :** vérification du registre du PR #85 avant de commencer. Même VM complète/chiffrée, File Editor et rollback sont déjà réussis, conservés.
- **Lacune nouvelle :** migration d’une sauvegarde complète entre DEUX processus QEMU HAOS réellement indépendants : identification boot ID de source/target, import de tar SHA-256 vérifié, restauration des octets et Core sur installation vierge.
- **Travaux :** scripts du relais local borné, workflow KVM séparé, deux tests réels et contrôle JUnit anti-faux-positifs, test de contrat CI. **Preuve obtenue avec exécution GitHub du SHA exact (6/6).**
- **GitHub :** [PR #86](https://github.com/GodsQuantum/Rusty-Home-Open/pull/86), branche test/fresh-second-haos-restore-20261024, premier commit fc26d357245d4a0d51de2362e072341b17bb9e60, correction CI 39b778274be83510f6a4ed9757e648dbc67638dd. Le premier run avait un YAML incorrect (sauts de ligne littéraux) ; problème corrigé immédiatement, sans cacher le résultat échoué. Statut test réel : SUCCESS run 40134632585, SHA exact 161307bd227b279c73df5a33da759a19ed4ac1ae, 3/3 source et 3/3 cible, vérifié dans les logs GitHub le 24 octobre 2026. Nouvelle preuve dans le registre JSON.
- **Blocage production :** même une réussite officielle HAOS→HAOS ne remplace pas le moteur Home Assistant/Core par Rusty Home ; 0/15 gates tant qu’aucune certification indépendante n’existe.


Ce journal est persistant dans **GitHub** et dans le dossier de travail **LXC700 `/home/arezki/Projets/Applications/`**. Chaque nouvelle session doit consulter ce fichier **avant** d'ouvrir un chantier ; toute PR doit ajouter un compte rendu daté et éviter les travaux déjà couverts dans le [registre des preuves](CERTIFICATION_EVIDENCE_REGISTER.md).

## 2026-10-24 — Vérification d'historique demandée : pas de tests redondants

- **Constat :** les sauvegardes complètes, leur restauration réelle dans HAOS, les sauvegardes chiffrées, le refus de mot de passe invalide, la restauration des octets File Editor et le rollback V1/V2 **étaient déjà prouvés**. La suggestion de « faire une première restauration complète » était redondante et a été corrigée.
- **Preuve vérifiée :** [run GitHub 40127707580](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40127707580), `8dde4a49315ee009c1c45341d5730f17668ea8f3`, **17 tests réels HAOS 18.3/QEMU réussis** ; [ancien run 39873207970](https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/39873207970), 16 tests précédents réussis.
- **Distinction documentée :** les restaurations ont lieu dans **la même VM QEMU** après modification de données, pas nécessairement sur une deuxième installation propre. Aucune déduction « seconde VM vierge prouvée ».
- **Ce qui a été créé pour éviter la répétition :** registre JSON contrôlé en CI, index lisible en français, liens depuis les pages principales et test automatisé de cohérence avec les fonctions HAOS et le workflow.
- **Projet / Git :** branche `docs/certification-evidence-register-20261024` depuis `main` `2b100ce75ca205e1d7ef8aeb2fad16ba2d52244d`. Index et tests publiés au commit `5fe54c3ef47ccee919a8b3493307e56e15a7a80a`, [PR #85](https://github.com/GodsQuantum/Rusty-Home-Open/pull/85). Résultat de fusion à confirmer uniquement après vérification du SHA final et des CI indépendantes.
- **Validations effectuées dans cette session :** `python3 scripts/test_certification_evidence_registry.py`, `scripts/test_public_certification_ledger.py`, `scripts/test_p8jm_file_editor_data_restore_contract.py` (4/4), `scripts/test_ha_backup_inspector.py`, `scripts/check_release_gate.py --expect-blocked`, vérification syntaxique Python et `git diff --check` : tous verts sur la branche isolée.
- **Non effectué :** aucun nouveau backup/restore, aucun travail sur HAOS familial, aucun test matériel ; le produit reste **NOT_PRODUCTION_READY (0/15)**.

## 2026-10-24 — Travaux déjà fusionnés avant cet inventaire

- [PR #80](https://github.com/GodsQuantum/Rusty-Home-Open/pull/80) : test QEMU HAOS du job asynchrone et son éviction, rejet sûr de mutation répétée. Ce n'est pas la preuve d'une clôture asynchrone positive.
- [PR #81](https://github.com/GodsQuantum/Rusty-Home-Open/pull/81) : installation synchrone native, postcondition officielle et clôture durable ACID, 17/17 scénarios HAOS validés.
- [PR #82](https://github.com/GodsQuantum/Rusty-Home-Open/pull/82) : préqualification de migration pour un nombre arbitraire d'applications, sans limite fixe de 9 exemples.
- [PR #83](https://github.com/GodsQuantum/Rusty-Home-Open/pull/83) : lecture des archives de sauvegarde sans suivre les symlinks, JSON duplicate-key reject.
- [PR #84](https://github.com/GodsQuantum/Rusty-Home-Open/pull/84) : document et test contractuel de restauration réelle des données File Editor.

## Prochaines tâches distinctes, sans redondance

**Mise à jour postérieure :** la restauration HAOS→HAOS sur une deuxième VM fraîche a été réalisée avec succès dans PR #86 (voir en-tête du journal). Les tâches véritablement manquantes concernent la migration de données depuis HAOS VERS LE MOTEUR Rusty Home, l'exécution des Apps tierces, les comptes et les vrais périphériques physiques, la sécurité et l'endurance de l'application. Les 15 gates de production restent non certifiés.
