# Home Assistant Core → states réels observés dans le Hub Moli/Rust

**25 octobre 2026. Statut : LAB-ONLY, 0/15 critères production.**

Cette tranche ne refait ni les restaurations déjà réussies, ni l'import de registre prouvé dans les PR #89–90. Elle utilise le même Core officiel 2026.10.2 (image OCI SHA-256 fixée) qui a enregistré les identités Core, puis fait produire à ce Core deux véritables objets de l'API interne `hass.states` via `State.as_dict()`.

## Nouveauté

Le module `hars-interop-moli::registry_stage::parse_core_state_snapshot` accepte un instantané JSON borné à 24 MiB / 50 000 entités, refuse les clés JSON en double (y compris en profondeur), identifiants répétés, timestamps manquants, valeurs non textuelles et caractères de contrôle. Dans `ha_registry_shadow`, les états sont associés **uniquement** aux entités déjà présentes dans le lot Core, dont l'identité et le SHA-256 ont été revalidés.

Le véritable driver du Hub Moli publie une valeur `Text` **uniquement lorsqu'un état actuel est fourni par Core**. Les états `unknown` et `unavailable` restent inconnus, comme les entités jamais observées. Les points demeurent `read=true, write=false` et un véritable `Hub::command()` doit être refusé. Aucun actionneur, webhook, token de production, réseau ou périphérique du foyer n'est sollicité.

Une nouvelle option CLI expérimentale `--lab-ha-core-state-snapshot-readonly` prend un fichier régulier privé (lien symbolique, hardlink et FIFO refusés), le lit sans suivre les liens et détruit son runtime en fin de test. Le flux Core et son `.storage` sont créés de manière synthétique dans une VM/conteneur CI éphémère.

## Preuves et limites

- Tests unitaires adversariaux du parseur, entités dupliquées, états indisponibles et état forgé hors registre.
- Moli Hub réel : 1 232 identités arbitraires disponibles, aucun point inscriptible. Un snapshot d'une valeur observée et une autre non observée produit exactement 1 état connu et 1 état inconnu.
- Le test Core officiel sauvegarde deux états `21.5` et `on`, exportés par `State.as_dict()`, puis la CLI doit confirmer **deux états connus dans le véritable Hub Moli**. Ce passage ne devient un acquis qu'après CI GitHub au commit exact.

**Ne pas confondre instantané avec miroir WebSocket continu :** ce protocole ne prouve pas la réception des changements, la réconciliation temporelle, le renouvellement de sessions, la durée de vie des états, le contrôle de services, les automatisations, ni la persistance des secrets. L'objectif final universel, y compris les intégrations tierces et le remplacement de Home Assistant Core, reste à certifier.
