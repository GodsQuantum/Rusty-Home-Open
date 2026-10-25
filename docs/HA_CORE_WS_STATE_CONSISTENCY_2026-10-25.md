# P8w — cohérence des états authentifiés entre Core, WebSocket et Moli

**25 octobre 2026 — laboratoire isolé, production interdite (0/15).**

## Réutilisation et raison du changement

Les PR #89–#94 sont fusionnées : registre officiel Core en Rust, périphériques du vrai Hub Moli, valeurs d'état, événements via hass-rs, exécutable borné, puis vraie authentification WebSocket par Home Assistant Core 2026.10.2 en Docker sans réseau. Ne pas recommencer ces essais ni les restaurations HAOS→HAOS déjà acquises.

Le contrôle de présence d'une connexion `get_states` pouvait réussir sans vérifier que le miroir d'événements avait reçu toutes les variations. Un socket vivant n'implique pas un état correct : le dernier état affiché par Moli pouvait être périmé si un événement n'avait pas été traité.

## Nouveau mécanisme

`crates/native-kernel/src/ha_event_convergence.rs` compare maintenant, lors de chaque vérification active, les états textuels des entités **enregistrées dans le lot Core**, les valeurs de la `ShadowSession` propre à l'époque WebSocket authentifiée et les échantillons réellement publiés par `moli-runtime::Hub`.

Le contrôle refuse un snapshot Core excessif, des IDs dupliqués ou des horodatages malformés, toute valeur contradictoire, une valeur ancienne présentée comme actuelle, une disparition non propagée et une divergence entre le Hub et Core. Une divergence révoque tout le miroir et retire les périphériques du Hub avant de rendre l'erreur. Une reconnexion doit impérativement repartir d'un nouvel abonnement et d'un instantané complet. Les valeurs `unknown`/`unavailable` sont traitées comme états inconnus, jamais comme commandes.

La lecture publique `read_verified()` consomme les événements en attente, obtient un `get_states` authentifié, vérifie la cohérence et consomme une seconde fois les événements. Ce protocole reste une observation non transactionnelle et n'est pas une garantie de synchronisation parfaite ou de détection des changements d'attributs qui ne modifient pas la valeur textuelle.

## Preuves et portée

- Tests Rust : divergence de valeur, snapshot plus ancien, état supprimé, réconciliation correcte, révocation complète du vrai Moli Hub et restauration par nouvelle session.
- Le scénario CI sur **véritable Core 2026.10.2**, déjà introduit en PR #94, est prolongé : au moins 8 événements `state_changed` successifs depuis Core, comparés à une lecture autoritative `get_states` ; Core doit également répondre `auth_invalid` à un jeton fictif erroné.
- Laboratoire Docker `--network none` partagé uniquement sur loopback, aucun port publié. Jeton uniquement dans une configuration éphémère protégée ; aucune donnée personnelle ni appareil connecté.
- Certification du SHA final GitHub obligatoire. Tant qu'elle n'est pas verte, les nouveaux scénarios ne sont pas acquis.

**Restant pour production :** supervision permanente et reprise automatique, tokens/permissions durables, TLS en réseau externe, changements d'attributs et registres, migration services/automatisations, compatibilité universelle des intégrations, sauvegarde HAOS→Rust→HAOS, essais matériel radio et endurance. Le release gate demeure 0/15.
