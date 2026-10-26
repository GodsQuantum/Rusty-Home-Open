# P8bj — révocation Moli après édition réellement persistée (LAB)

## Lacune distincte
P8bi #142 prouvait l'import d'une automatisation HAOS fictive et désactivée vers un graphe Moli synthétique. La prétendue révocation ne faisait que tester changed.is_approved() sur une copie jamais sauvegardée. Aucun arrêt réel de l'engine ni résistance au redémarrage n'étaient démontrés.

## Test réel
Le moteur upstream moli-automation (rudy-io/moli-os d54c3de46aa453691f0c96ddc42e853a8a964178) approuve une première version, déclenche un faux actionneur, puis reçoit une modification de logique via un Actor Agent non humain, réellement sauvegardée dans automations.json. Le test vérifie ensuite is_live=false, refus de l'empreinte périmée, zéro nouveau run et aucune action après un nouvel événement. Un nouveau Hub/engine recharge le stockage et vérifie encore les quatre propriétés. Le test Rust ciblé et les contrats Python existants exigent ces preuves.

## Preuves CT700 (26 octobre 2026)
- Test Rust actual_moli_persisted_agent_edit_revokes_approval_across_restart : 3/3 exécutions PASS (1 test chacune, environ 0,80 s).
- Deux tests unitaires existants automation_moli : 2/2 PASS avant ajout.
- Build natif Rust : PASS ; Clippy strict -D warnings : PASS ; cargo fmt et git diff --check : PASS.
- Binaire réel --lab-ha-automation-moli : PASS avec persisted_edit_inactive=true, stale_approval_rejected=true, no_run_after_edit=true, restart_kept_edit_inactive=true, hardware_touched=false, production_ready=false.

## Portée et livraison
Pilote synthétique seulement, aucun VM100, appareil, radio, secret, montage ou service de production. Le rôle humain du test est simulé et n'est ni une signature ni une certification indépendante. CI GitHub P8bj au HEAD exact non exécutée et PR non poussée faute d'accès GitHub en écriture depuis CT700. P8bi #142 était distinctement vert 13/13 à 2293d85b0d2c0c1ef514e2be75ec4f25f48b75f9 et fusionné 6eeda5b718f3bd6b214053808e082841bd7494b9. Statut produit NOT_PRODUCTION_READY, 0/15 gates indépendants.
