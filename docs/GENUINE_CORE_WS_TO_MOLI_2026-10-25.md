# P8v — WebSocket du véritable Home Assistant Core vers le Hub Moli/Rust

**25 octobre 2026. En cours de certification CI : laboratoire isolé, 0/15 critères de production.**

## Pourquoi cette tranche est différente

Les PR #89, #90 et #91 ont déjà prouvé l'export du véritable registre Core, l'import durable et les états observés dans le Hub Moli. Les PR #92 et #93 utilisent ensuite un véritable WebSocket local mais avec un serveur **fictif**. P8v remplace ce serveur fictif par un **Home Assistant Core 2026.10.2 officiel**, image OCI figée par digest, avec le système d'authentification officiel, des comptes fictifs, une véritable config entry, un registre Core, des états Core et la route authentifiée /api/websocket.

## Architecture du laboratoire

Un premier conteneur Core officiel tourne sans réseau (`docker --network none`). Un second conteneur Ubuntu exécute le binaire Rust précompilé avec `--network container:<Core>` pour rejoindre **seulement l'espace réseau du Core**. Aucun port sur l'hôte, aucune connexion Internet, aucun routage extérieur. Docker refuse toutes les capacités additionnelles, le mode privilégié, et les conteneurs sont non-root et read-only.

La configuration Core est créée dans un répertoire temporaire appartenant uniquement à l'utilisateur CI. Core crée un utilisateur administrateur fictif dans son propre auth manager, génère un jeton temporaire et l'écrit dans un fichier propriétaire privé, ainsi qu'un registre d'identités. Rust ne lit que ce dossier éphémère et le lot redb préparé pour ce test. Le jeton est transmis au processus Rust **par environnement depuis le second conteneur**, jamais par un argument visible dans la commande ni dans un résultat JSON.

Le runner utilise le client existant `hass-rs`, s'abonne à `state_changed` avant `get_states`, reçoit des événements créés par `hass.states.async_set` du vrai Core, vérifie le changement observé dans le vrai Hub Moli, refuse toute écriture, puis sort sous 20 secondes. L'identité est connue via le registre importé ; `ha_registry_stager` effectue un rollback puis vérifie l'absence de lot actif. Les conteneurs et dossiers temporaires sont détruits après le test.

## Contraintes et état de preuve

Le script `scripts/run_p8v_genuine_core_ws_to_moli.sh` s'exécute uniquement en utilisateur non-root GitHub Actions. Le workflow `core-interop-public.yml` ajoute un job séparé avec vérifications préalables et nettoyage. Le contrat `test_p8v_official_core_ws_contract.py` interdit les ports publiés, les réseaux sortants et les déclarations de production.

**Ne pas déclarer le test réel réussi avant le marqueur du run CI exact** `P8V_GENUINE_OFFICIAL_CORE_WEBSOCKET_AUTH_TO_ACTUAL_RUST_MOLI_PASS`.

Même si le test est vert, ce sera une preuve d'intégration Core authentifiée sur un appareil fictif **isolé**, pas une migration de production. Restent : supervision TLS/renouvellement des jetons, pertes et reconnexions durables, valeurs historiques, automatisations/actions, plugins HACS réels dans ce runtime, accès matériel, restauration HAOS→Rust→HAOS et certification des 15 gates.
