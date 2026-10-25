# P8u — exécutable de laboratoire Core WebSocket → Hub Moli

**25 octobre 2026. Statut : laboratoire uniquement, production 0/15.**

## Préexistants à conserver

Les PR #89, #90 et #91 ont déjà validé le registre et les états de Core vers Rust/Moli. La PR #92 a ensuite raccordé le client WebSocket hass-rs déjà existant à un driver Moli en lecture seule, avec file bornée, époques, invalidation et test de transport. Ne pas refaire les restaurations HAOS→HAOS, ni créer un nouveau protocole WebSocket.

## Nouvelle fonctionnalité

Un **exécutable de laboratoire réellement invocable**, dans le binaire Rust natif, combine dans un seul processus : ouverture d'un lot Core déjà approuvé en redb, authentification WebSocket locale, abonnement aux changements AVANT la prise d'instantané, synchronisation au Hub Moli, observation d'un nombre borné d'événements, vérification explicite d'un état attendu, puis extinction. Il réutilise le même EventConverger que la PR #92.

Commande prévue depuis le dépôt en utilisateur non-root :

- Flag : --lab-ha-ws-to-moli
- Paramètres : chemin privé sous le dossier runtime du projet, URL loopback ws://127.0.0.1:PORT/api/websocket, identifiant Core connu, valeur attendue, nombre de changements (1 à 64)
- Jeton : variable d'environnement HARS_LAB_WS_TOKEN, jamais un argument de ligne de commande
- Résultat : une preuve JSON sans secret, indiquant les changements reçus, le nombre de valeurs vérifiées, read_only=true et production_ready=false.

Ce runner s'arrête avec échec si la source est invalide, si l'identifiant ne figure pas dans le registre, si la valeur reçue ne correspond pas, si l'authentification échoue, si le transport se coupe, si une sonde échoue ou si le délai global de 20 secondes est dépassé. Il n'envoie aucune action d'appareil. Il utilise exclusivement les contrôles de lecture de Home Assistant.

## Évidence

Le test utilise un **véritable serveur WebSocket éphémère sur loopback** qui valide la séquence auth → subscribe_events → get_states → state_changed → get_states et interdit toute commande non lue. La valeur est validée par le véritable Hub Moli. Un test négatif refuse un identifiant hors registre avant l'établissement de la connexion. L'option binaire doit également passer le contrat de refus des chemins arbitraires et des jetons manquants.

Une telle preuve ne remplace pas une expérience avec le **véritable serveur WebSocket de Home Assistant Core**, ni un service de production durable. Il reste à développer TLS/auth robuste, superviseur de reconnexion, persistance des valeurs contrôlée, services, automatisations, matériel, sauvegarde/restauration HAOS→Rust→HAOS, et l'audit externe. Ne pas certifier 1 des 15 critères complets sur cette seule base.
