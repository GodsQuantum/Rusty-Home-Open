# Premier raccord réel des identités HA au Hub Moli/Rust

**25 octobre 2026. Statut : LAB-ONLY, pas de remplacement de Core, 0/15 gates de production.**

## Origine et non-redondance

- Les restaurations officielles HAOS→HAOS (même VM et seconde VM fraîche) sont déjà certifiées en QEMU ; ne pas les rejouer.
- La PR #89 a validé avec le véritable Home Assistant Core 2026.10.2 un export de `.storage/core.entity_registry` vers l'ancien crate `hars-interop-moli` avec base privée redb, activation CAS et rollback durable.
- Le nouveau module `crates/native-kernel/src/ha_registry_shadow.rs` exploite précisément cette base **pour créer des périphériques read-only dans le véritable Hub Moli OS**, pas dans un second moteur factice.

## Garantie technique

Chaque entrée Core active est re-projetée en types `moli_core::DeviceId` et `PointId` ; le driver rejette les collisions, les identités modifiées, les états prétendument connus et tout transfert d'identité hors de l'instance `ha-compat`. Le point Moli `state` est lisible mais **strictement non inscriptible**, son état initial est ABSENT et non une valeur fabriquée. Les commandes reçues inopinément sont refusées. La méthode `Hub::command` native est volontairement testée pour confirmer le rejet effectif.

Le protocole ne transforme **aucun service Home Assistant en commande réelle**, ne pilote aucun actionneur et ne stocke ni token ni secret de foyer. Aucune interaction avec HAOS familial, aucune installation sur machine physique.

## Preuves

- 2 tests Rust supplémentaires sur le vrai Hub Moli : 2 entités puis 1 232 identités multi-fournisseurs, validation de 1 232 périphériques, aucun état inventé, commande refusée.
- La CI Core officielle importera des entités créées dans un vrai conteneur Core vers le registre Rust déjà fusionné, puis les matérialisera **dans un vrai Hub Moli** avant rollback.
- L'exécutable native-kernel expose uniquement l'option isolée `--lab-ha-registry-shadow-readonly`, non utilisable pour une migration réelle de production.
- Ne marquer la preuve réelle Core→Moli comme acquise **qu'après GitHub Actions entièrement verte sur le SHA final**.

## Ce qu'il reste pour remplacer Core/HAOS

Associer des données d'états effectivement observées par WebSocket et des registres complets aux objets Moli, conserver la causalité / monotonicité des changements, importer scripts/automatisations et secrets protégés, puis exécuter des services autorisés en parité avec Core. Certifier multiutilisateurs, matériel radio réel, HACS, rollback HAOS→Rust→HAOS et endurance. Le compteur de release reste 0/15.
