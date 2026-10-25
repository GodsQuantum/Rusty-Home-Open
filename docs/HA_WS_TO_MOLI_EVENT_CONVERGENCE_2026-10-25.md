# P8t — événements Home Assistant WebSocket vers le vrai Hub Moli

**25 octobre 2026 — laboratoire uniquement, production 0/15.**

## Travaux antérieurs conservés

PR #89 : registres Core stockés en redb avec rollback. PR #90 : périphériques en lecture seule dans le vrai Hub Moli. PR #91 : états issus du véritable Core puis projetés dans Moli. Le module ha-compat possédait déjà le client hass-rs 0.5.0, un abonnement aux changements avant le premier instantané, les époques de sessions, le traitement des horodatages et la détection de débordement.

## Nouvelle tranche

Le module crates/native-kernel/src/ha_event_convergence.rs réutilise ce client et la même base d'identités pour répercuter les événements de Core dans le vrai Hub Moli. Une file mpsc bornée synchronise les valeurs et reçoit un acquittement après publication. Les états obsolètes ne remplacent pas les plus récents ; un événement contradictoire invalide tout le miroir. La suppression ou la valeur unavailable purge l'ancien état ; un ID non enregistré ne crée pas de périphérique. Toutes les écritures demeurent interdites par Moli.

Le test du socket réseau de laboratoire a révélé que hass-rs ne ferme pas toujours immédiatement son récepteur d'événements lorsque TCP est interrompu. Une vérification active de transport par get_states avec délai borné entraîne la suppression de tous les périphériques Moli en cas d'échec. La lecture surveillée traite d'abord les événements en attente ; le bail de fraîcheur demeure borné à 30 secondes. Aucun handle brut du Hub n'est exposé aux consommateurs.

## Validation, portée et limites

Tests unitaires : propagation, horodatages plus anciens, conflits, mauvaises générations, suppression, disponibilité, IDs inconnus, refus des commandes et reconnexion. Test WebSocket loopback complet : authentification, souscription, lecture de l'instantané, événement, valeur du Hub, fermeture réseau et révocation par sonde.

Cette preuve utilise un serveur WebSocket **synthétique local**. Le vrai Core a déjà été testé pour les étapes de registre et d'instantané, mais pas encore comme serveur producteur de WebSocket de cette nouvelle tranche. Ce n'est ni un daemon HAOS de production, ni une synchronisation garantie sans pertes, ni un substitut aux automatisations, aux services ou aux intégrations. Le besoin de sondes supervisées, de TLS/auth sécurisée, de restauration Core et de tests longue durée demeure.

L'ensemble doit réussir la CI GitHub sur le SHA exact avant d'être considéré fusionnable. Aucun service Cloud9 familial n'est modifié.
