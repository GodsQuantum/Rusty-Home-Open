# HA Core → Moli/Rust : migration incrémentale des identités

**25 octobre 2026 — première tranche de migration native, lecture seule.** Statut production : **NOT_PRODUCTION_READY, 0/15 critères**.

## Ce qui existait déjà

Ne jamais refaire les tests HAOS→HAOS. Le projet possède déjà le rapprochement d'identités HA→Moli dans `crates/interop-moli`, les restaurations officielles HAOS dans une seule VM (17/17) et vers une deuxième VM HAOS vierge (6/6). Voir [registre](CERTIFICATION_EVIDENCE_REGISTER.md). Ces preuves ne démontrent PAS l'import HAOS→Rust.

## Nouveau résultat distinct

`crates/interop-moli/src/registry_stage.rs` et `src/bin/ha_registry_stager.rs` fournissent le premier **stockage durable d'un registre de vrais identifiants HA, sans exécuter de commandes**. Les données sont lues depuis `core.entity_registry` officiel (fichier `.storage`) ; les identités sont converties avec la projection Moli déjà existante. Chaque batch est ordonné, SHA-256, borné à 24 MiB/50 000 entrées, écrit atomiquement en redb dans un dossier privé 0700 (base 0600), puis activé uniquement avec comparaison explicite de l'identifiant actif. Le rollback est atomique et persiste à travers plusieurs processus Rust.

Le parseur refuse clés JSON dupliquées, collisions d'identifiants Moli, ID entité dupliqué, schéma inconnu, liens symboliques, hardlinks et FIFOs. Pour chaque entrée, **l'état d'appareil reste explicitement inconnu** (`state_known=false`). Les secrets d'intégration, tokens OAuth, valeurs de sondes et opérations d'écriture ne sont pas importés ni exécutés. La CLI est uniquement `--lab-readonly` et n'expose pas de serveur réseau. Toute corruption ou divergence de version est refusée.

## Validation

- `cargo test --locked -p hars-interop-moli --all-targets` : 13 tests Rust, dont 5 du nouvel importeur ; `cargo clippy -D warnings` strict.
- `python3 scripts/test_ha_registry_stager.py` : 1 232 identités multi-fournisseurs fictives, préparation persistante, activation CAS, remplacement, rollback dans plusieurs processus et tentatives hostiles refusées. **Ce n'est pas le vrai Core.**
- `scripts/test_p8q_real_core_registry_export.py` : script exécuté dans un conteneur **officiel Core 2026.10.2 immuable**, sans réseau et sans root, crée de vraies entités de registre, utilise le composant de stockage officiel, puis fournit son fichier à la CLI Rust.
- `scripts/run_p8q_real_core_to_native_registry.sh` : en CI dédiée, importe ce fichier créé par Core, vérifie les identités persistées et leur rollback. Ne déclarer ce trajet de bout en bout **réellement validé** qu'une fois la CI GitHub du SHA exact réussie.

## Limites de la couche `.storage`

`.storage/core.entity_registry` est une représentation interne et évolutive, **pas une API stable d'échange**. L'objectif production sera un exporteur basé sur les API officielles de registres et le contrat versionné de migration, suivi de données Core, services, secrets et automatisations protégés. Les évolutions 2026 des registres rendent une réécriture manuelle de leurs structures particulièrement risquée.

Références : [API et dépréciations du registre des appareils, août 2026](https://developers.home-assistant.io/blog/2026/08/24/device-registry-follow-up-changes/), [utilisation des config entries officielles](https://developers.home-assistant.io/blog/2024/02/12/async_update_entry/), [Core real entity registry](https://github.com/home-assistant/core/blob/dev/homeassistant/helpers/entity_registry.py).

## Prochaine tranche de certification — ne pas confondre avec celle-ci

Lier les ID réellement importés à un runtime Rust vivant, puis prouver un miroir cohérent des états par WebSocket/Core, les automatisations et les commandes autorisées, et enfin une **migration complète HAOS→Rust + rollback vers HAOS**. Il faudra une stratégie cryptographique pour les secrets et tester plusieurs schémas d'intégrations. Tant que la projection reste une base de lecture seule, cela ne vaut **aucune certification de production**.
