# Rusty Home — journal de développement et de certification

## 2026-10-24 — P8p, nouvelle validation inter-VM (proposée, CI non encore prouvée)

- **Anti-redondance :** vérification du registre du PR #85 avant de commencer. Même VM complète/chiffrée, File Editor et rollback sont déjà réussis, conservés.
- **Lacune nouvelle :** migration d’une sauvegarde complète entre DEUX processus QEMU HAOS réellement indépendants : identification boot ID de source/target, import de tar SHA-256 vérifié, restauration des octets et Core sur installation vierge.
- **Travaux :** scripts du relais local borné, workflow KVM séparé, deux tests réels et contrôle JUnit anti-faux-positifs, test de contrat CI. **Ne PAS annoncer comme réussi avant les exécutions GitHub du SHA exact.**
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

Une éventuelle **restauration sur une deuxième VM fraîche** (indépendante de la source), la migration de données arbitraires vers le moteur Rust et les tests de périphériques physiques sont **des tests différents** du round-trip déjà effectué. Ils ne doivent être entrepris qu'après inventaire des preuves et de la stratégie de réversibilité. Les 15 gates de production restent non certifiés.
