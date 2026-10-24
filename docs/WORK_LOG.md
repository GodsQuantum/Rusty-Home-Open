# Rusty Home — journal de développement et de certification

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
