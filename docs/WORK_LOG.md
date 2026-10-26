# Rusty Home — journal de développement et de certification

## 2026-10-26 — P8aw custom safe App volume path, real HTTP registry state (CI pending)

- P8av #126 real Redis and Memcached independent rootless OCI lifecycles 10/10 GitHub checks MERGED 6ed03afa6d9a55a28cd4a51d28d48c21480fe57d. Prior proof preserved, no redundant re-certification.
- Native generic OciApp now supports validated configurable absolute container data volume destination via render_at, still Podman-managed named volume only and mandatory non-root UID. Refuses relative/traversal/control strings and mount over sensitive system locations. Existing default /data remains unchanged and old fixtures pass.
- P8aw tests genuine official CNCF Distribution Registry 3.1.2 running nonroot under native Rust management: optional 127.0.0.1:18884 mapping to internal 5000 only; named volume mounted /var/lib/registry; real Docker Registry HTTP blob upload, digest verification/GET and persistence after systemd-managed stop/restart. Requires real exact-feature GitHub CI. NOT a live family registry, authentication/remote network release, encrypted offsite backup, complete App Store support or HAOS replacement. 0/15.


## 2026-10-26 — P8av two genuinely distinct simultaneous OCI Apps (CI pending)

- P8au #125 genuine Redis + incompatible BusyBox healthcheck automatic image rollback 26/26 CI green, merged 05008d4cb9d2d577bec1567ea3bcda67f1faaa2e. Prior evidence pinned, do not repeat as new proof.
- New *unchanged* generic Rust App manager compatibility lab runs two independent real official OCI packages, Redis 7.4 Alpine and Memcached 1.6 Alpine, under rootless Podman/systemd on isolated GitHub Ubuntu. Image tag is resolved to exact runtime manifest sha256 before each install. User 999 vs 11211, isolated network none/readonly root, no host ports, independent named volumes.
- Real Redis durable SAVE/GET and Memcached volatile SET/GET/version protocol tests; stop/restart Memcached while Redis stays alive and retains data. Independently uninstall Memcached while Redis continues; preserve both Podman volumes. The real OCI images differ in app implementation, lifecycle and storage semantics: this is first multi-App proof beyond Redis alone, NOT a whitelist of 2, 9 or any fixed number. Requires exact-branch GitHub CI before certification. Still missing HA Supervisor metadata/permissions/configuration parity, production RBAC, persistent host lifecycle and cutover, 0/15.


## 2026-10-26 — P8as optional local-only TCP ingress for generic OCI Apps (CI pending)

- P8ar native Rust manager actual Redis install/start/stop/uninstall/rootless persistent state verified in real GitHub Ubuntu on run 40291069681; exact-head 23 CI checks required before #121 merged. Do not reimplement P8ar.
- Introduces generic opt-in AppNetwork::LoopbackTcp with explicit host and container port, rejecting host ports under 1024 and invalid guest ports. Default remains AppNetwork::Offline / Network=none; no 0.0.0.0/public publish. Uses native Podman 4.9 Quadlet Network=slirp4netns and PublishPort=127.0.0.1, not custom proxies. The Rust native manager exposes explicit --local-tcp only with laboratory flag.
- Separate disposable GitHub test runs a genuine Redis App and tests actual host 127.0.0.1:18879 reachability, Podman port listing and strict read-only/rootless resources; will not claim proof before exact-head CI. IMPORTANT: slirp4netns can permit OUTBOUND egress; this is a host-side INGRESS limitation, NOT network egress isolation, production remote ingress, auth or completed Home Assistant App integration. No Cloud9 service change, 0/15 production.


## 2026-10-26 — P8ar real native Rust OCI App manager commands (CI pending)

- Earlier P8ap #118 actual rootless Redis systemd start/restart/persistence 21/21 CI merged 071b85bac4f887843be3a2f47d25e44ddcc03bd1. P8aq #120 real offline Podman volume export/import, independent restored Redis and corrupted archive refusal 11/11 CI merged 7cf1d3ad3fb74d9e7d8537f171e500b94d86eac0. DO NOT REDO THESE TWO PROOFS.
- New genuine Rust binary crates/native-hub/src/bin/oci_app_manager_lab.rs exposes explicit laboratory install/start/stop/remove of arbitrary safe OCI App slugs via the upstream native Podman+systemd rootless system. App digest must be SHA256 pinned and image already local. Install writes two new private Quadlet descriptor files, reloading systemd without auto-start; start and stop invoke unprivileged systemctl --user. Removal stops and unloads descriptors WITHOUT deleting App state volume. No fixed App list.
- Strictly restricted to non-root GitHub ephemeral CI with second explicit lab opt-in. Reject symlink/foreign ownership/world-writable unit directories and files, mutable images, UID0, malformed slugs, missing manager install records, unauthorized commands. Real official Redis OCI end-to-end operation CI p8ar-real-oci-manager.yml required at exact SHA.
- No real cloud9 or household access, production authentication/RBAC, app version upgrade transaction or encrypted migration; still 0/15 release gates fully qualified.

## 2026-10-26 — P8aq real independent OCI App data backup and restore (CI passed 11/11, merged #120)

- Already verified: #118 actual rootless Redis App start/stop and persisted data, 21/21 exact-head CI, merged 071b85bac4f887843be3a2f47d25e44ddcc03bd1. DO NOT REDO.
- P8aq extends the same Redis + systemd rootless fixture only under an additional explicit laboratory environment. After stopping Redis writer, native Podman volume export writes private tar, enforces SHA256, corrupts a second copy and requires checksum rejection. The original Redis App restarts and its state is intentionally changed. A FRESH differently named Podman volume imports the checksum-verified archive; a new Rust-generated Quadlet starts the independent restore App; prior backup value is checked while primary still has new value.
- Disposable GitHub Ubuntu only, home HAOS untouched. Not generic HAOS migration/cutover, signed backups, offsite encryption, multi-volume transaction consistency or long endurance. Production 0/15.


## 2026-10-26 — P8ap genuine rootless native Podman App lifecycle (CI pending)

- P8ao #117 generic OCI native Quadlet verified in actual Podman 4.9 generator, 20/20 CI, run 40264974609, merged b8d322bc8c37e3c4302f7eb08e45c76dbebccd7b. No fixed App allowlist.
- P8ap extends manifest with optional explicitly nonroot numeric UID and native named-volume :U ownership; validates no root UID/injection; no host path/network access.
- New genuine Redis 7.4-alpine OCI test in isolated GitHub Ubuntu: pre-pull then pin exact image digest, generate rootless Quadlet from Rust, systemd --user starts/stops and restarts service, redis-cli SET+SAVE then checks persisted bytes, default network none and read-only image. Must pass real exact SHA before certification. No production release gate yet (0/15).


## 2026-10-26 — P8ao generic native Podman Quadlet OCI App foundation (CI pending)

- Distinct from P8an automatic QEMU crash-restart CI already in progress, Rust now renders generic OCI App manifests into native Podman Quadlet container+volume units, supporting arbitrary safe slugs and sha256-pinned registries; nine Apps are representative tests, never a whitelist.
- Secure defaults: no networking, read-only root, no new privileges, no Linux capabilities, bounded resources, pre-pulled images and persistent named volumes. Opt-in private directory only; no actual family app deployment.
- Official Podman generator CLI on ephemeral Ubuntu must validate real parser/generator. This does not yet implement lifecycle/auth/backup parity. All ten host proofs preserved, release 0/15.


## 2026-10-26 — P8am native Type=notify and QMP heartbeat (CI pending)

- Type=exec reported active before QEMU control was ready. Prior P8aj real HAOS systemd and P8ak package proofs are complete and not retested as new results.
- Rust uses pinned sd-notify 0.5.0; Type=notify READY after real QMP greeting, WatchdogSec=30s heartbeat conditional on QMP, STOPPING on shutdown. Failed notify returns via QEMU cleanup.
- New official HAOS KVM CI checks WatchdogTimestampMonotonic advancement during two systemd start/stop cycles and unchanged private QCOW2 backing. CI must pass at exact SHA; no household installation, 0/15 gates.


## 2026-10-26 — P8al real guest manager in signed APT index (CI pending)

- First exact-head CI found test-fixture APT source mismatch: apt-get used a temporary isolated sources.list, while the newly added apt-cache probe consulted default sources and returned "No packages found". Fixed apt-cache to use the SAME explicit trusted source options. Retest required; package index was already verified to contain both native versions.


- P8ak PR #112 11/11 PASS merged cae2cde2ce1e08eb86bd5a2b0465212be07845cf; real Ubuntu package 40255870443 and official HAOS systemd KVM 40255870364. Do not duplicate completed proofs.
- Modified existing P12b ephemeral signed-APT repo generator to include both actual native guest .deb versions. Added independent guard for signed metadata discovery of two versions in an offline Ubuntu client, preserving explicit tampered InRelease refusal.
- CI-only disposable signing key; no production trusted key or HAOS household installation. Product 0/15.


## 2026-10-26 — P8ak native Rust guest Debian lab packaging (CI pending)

- Completed P8aj #111 real systemd official HAOS guest laboratory proof 10/10 PASS, GH 40254861417, merged 4a1377341eabd44bc81334705465a5ebf33cf6c4. No redo as a fresh certification.
- New reproducible native manager Debian .deb (not the old synthetic read-only CLI package), lab-only versioned revisions, /usr/libexec service path, systemd unit and sysusers definition. The unit and package have no automatic start/enable. Real ephemeral Ubuntu dpkg install/update/downgrade/purge and updated actual systemd KVM workflow required at feature SHA.
- No Cloud9/HAOS family changes, no real radio/device access, no signed production release, 0/15 production gates.


## 2026-10-26 — P8aj genuine systemd start/stop of real HAOS guest (CI pending)

- P8ai #110 genuine Core frontend HTTP 16/16 passed and merged e891401d65d6f954afb809bd7ff9d7312c7b3e7f. Preserve verified proof; don't duplicate.
- New opt-in sample systemd service, owner-only COW data, dedicated account and KVM sandbox. No Install section, never auto-enable.
- New real official HAOS 18.3 GitHub workflow tests two actual systemd start/stop cycles, no orphan guest, same disk overlay and pristine pinned backing, no Cloud9 or family touch. Exact SHA CI required. Production 0/15.


## 2026-10-26 — P8ai actual Home Assistant Core frontend through Rust QEMU (CI pending)

Added a native Rust localhost read-only HTTP request to Core through QEMU. New real HAOS 18.3 KVM proof waits up to bounded limit for HTTP success, rather than mistaking P8ah TCP listener availability for Core readiness. Does not authenticate, mutate entities, or expose LAN services. Release 0/15.


## 2026-10-26 — P8ah native Rust HAOS frontend loopback forwarding (CI pending)

Rust guest host now optionally exposes only QEMU localhost 127.0.0.1:18180 forwarding to official HAOS guest TCP/80, with a real host TCP listener probe. The default lab mode still exposes no ports; no public binding, HA Core token or radio. Genuine KVM CI required before certification; login UI health and production are not yet proven.


## 2026-10-26 — P8ag continuous Rust-managed HAOS guest (CI pending)

New Rust guest host process lifecycle, existing COW disk, private QMP, bounded shutdown and QEMU reap. Exact official HAOS KVM CI pending. No installation to the family system, no radio ownership and no production gate certified.


## 2026-10-26 — pinned results of P8ac–P8af (all green and merged)

- Four non-duplicative official pinned HAOS 18.3 KVM proofs: Rust QEMU boot (P8ac, 18/18), actual Supervisor and Core CLI health (P8ad, 11/11), persistence of COW data across QEMU hard restart (P8ae, 12/12), and an entirely new Rust process reattaching the old guest storage while rejecting concurrent ownership (P8af, 13/13).
- Exact runs 40243098816, 40243728521, 40247000475 and 40247834775; PRs #103–#106 all merged at main d8c5bee817c7b7e8ef5ea6af24d62d366fbe610a. Preserve outcomes and do not retest as new certifications. Exact feature SHA, immutable CI markers and limits in config/haos-rust-guest-evidence.json.
- No changes on the family HAOS/Cloud9 host deployment or actual radios. Full Core API, Apps lifecycle, native Rust service daemon, signed distribution, real radio fencing, 72h soak and independent review still missing. Release 0/15.


## 2026-10-26 — P8af Rust process reattachment (CI pending)

New resume command reopens exact QCOW2 backing source and tests exclusive Rust manager ownership. Independent genuine KVM two-process proof and negative flock test must pass GitHub. Not production.


## 2026-10-25 — P8ae persistent HAOS guest under Rust (CI pending)

- New Rust stateful guest orchestration: atomic/private fixed qcow2 COW overlay from immutable official HAOS backing image, flock exclusive single owner, repeated KVM guests without ephemeral -snapshot, same real official HAOS guest data disk.
- End-to-end new failure scenario: fake HAOS share marker written/synced by serial root in first guest, SIGKILL QEMU process, independent new QEMU process verifies sha256sum of persisted marker. Source image unchanged. Real KVM CI required at exact SHA before proof.
- No real family radio, keys or automations. Core/Supervisor health P8ad remains verified in PR #104 (real HAOS QEMU 40243728521, 11/11 checks); still NOT_PRODUCTION_READY 0/15.


## 2026-10-25 — P8ad real Supervisor/Core response under Rust-owned HAOS VM (CI pending)

- Follow-on from merged P8ac #103 (KVM serial boot), not repetition of its boot test. Native Rust now drives official guest serial console, authenticates local root in disposable HAOS, independently requests official Supervisor and Core CLI info (read-only) and refuses success unless both return result=ok.
- Negative unit cases, structured redacted booleans, bounded retry/timeout and no image mutations. New genuine HAOS 18.3 KVM workflow; verify its exact CI SHA before claiming healthy Core or Supervisor. Persistent guest/App service running and production RBAC/radio ownership are still unproved; 0/15 production.


## 2026-10-25 — P8ac official HAOS guest from native Rust (CI pending)

- New capability: a real Rust process orchestrates official HAOS QEMU KVM boot with pinned image digest, no original image writes, fixed machine args, no radio passthrough, bounded cleanup. Guest still runs original Core/Supervisor/Apps; this is not a native Rust replacement of their functions.
- Isolated GitHub QEMU lab needed at exact source SHA. Detecting the official HAOS login console is an OS boot proof, NOT evidence that Core and Apps are healthy.
- Reuse real HAOS backup/import/rollback P8p-P8ab proofs. Production remains 0/15; no family HAOS touched.


## 2026-10-25 — P8ab, reversible whole-archive escrow between actual HAOS VMs (lab verified)

- New native Rust opaque, content-addressed and fsynced private HAOS archive escrow; never parses secrets, preserves all bytes including encrypted payload, hashes on ingest/export and denies symlinks, corrupt blobs, weak file permissions and overwrite.
- Reuses existing proven P8p QEMU 18.3 source and separate fresh target; adds an actual durable Rust vault in between. The original guest archive is replaced by Rust-exported bytes before second VM restore.
- Local synthetic tests positive and hostile PASS. The genuine two-VM run PASSED: GitHub 40240344194, SHA 58cbfe3023532b7acca68123641a58692fab541d, PR #101 merged at 4757377d5d773844f6e278af4712597d50cd799a; 3/3 first VM, 3/3 second VM; 12 checks PASS.
- Core/Apps/radios DO NOT run in Rust, whole product stays 0/15 production, no family HAOS involved.


## 2026-10-25 — P8aa, vrai tar HAOS vers stockage Rust (laboratoire certifié)

- Anti-redondance : réutiliser les preuves HAOS→HAOS 17/17 et 6/6 et les PR Core→Rust/Moli #89–#98, sans les renommer en certification native.
- Fonction nouvelle : Rust ouvre le tar complet non chiffré, vérifie les en-têtes, le hash, les chemins, les limites, le CRC gzip, récupère seulement core.entity_registry, stocke les identités en redb durable et offre activation CAS / rollback.
- Tests locaux : archives factices valides/hostiles, refus chiffrement, liens et gzip tronqués, aucun service du domicile.
- Test CI distinct : vrai Core 2026.10.2 emballé dans un tar synthétique, plus vraie sauvegarde Supervisor HAOS 18.3 créée dans une VM jetable et importée vers Rust (preuve RÉUSSIE au run 40238122942, SHA 9c6e3220f433153c0dc28ef0d2549bd52a345a8f, merge d297bbbd29ac313723b54794ad27aceaf0636166).
- Dépendances contrôlées : Cargo.lock +9 crates, source SPDX 226 paquets / 522 relations ; pas de clearance légale. Production 0/15.
- Dossier : docs/HAOS_TO_NATIVE_BACKUP_STAGE_2026-10-25.md.


## 2026-10-25 — P8z, sondes Core/WS/Moli et course de livraison

- **Anti-redondance :** PR #97 fusionnée (c8c6a73) après 9/9 CI au SHA feature 141bf49, preuve P8y durable ; anciennes restaurations HAOS→HAOS conservées.
- **Lacune :** sans drainage après get_states, une mise à jour seulement d'attributs peut arriver dans le relais avant le retour du snapshot et entraîner une révocation intempestive.
- **Correction :** lecture en boucle strictement bornée (trois snapshots max, deux intervalles 30 ms), drainage avant/après chaque snapshot, vérification structurale Core/WS/Moli et purge à la troisième divergence. Pas de reconnexion ni de write.
- **Compteur :** les événements consommés pendant la sonde sont comptés dans le runner de laboratoire.
- **Preuves :** nouveau test WS loopback des deux cas livré/perdu et checks de l'espace Rust ; CI exacte à vérifier ; production toujours 0/15.


## 2026-10-25 — P8y, audit du cas attribut-seul WebSocket

- **Réutilisation :** main au commit 2fdcb77, PR #89–#96 déjà intégrées ; preuves de migration et restaurations HAOS conservées.
- **Nouveau bug :** get_states Core peut renvoyer un état textuel identique mais last_updated plus récent si un state_changed d'attribut a été perdu.
- **Correctif laboratoire :** égalité stricte des timestamps ET des attributs JSON officiels Core / miroir WS avant d'autoriser une lecture Moli ; toute divergence révoque le miroir et tous les périphériques.
- **Qualification :** test de régression dédié, CI au SHA exact à vérifier. Connu : déclenchement fail-closed possible pendant une course entre snapshot et livraison WS ; reconnexion/réconciliation durable encore absente. Production 0/15.


## 2026-10-25 — P8x, registre canonique pour sept certifications Core→Rust/Moli sans répétition

- Après fusion des PR #89–#95, preuves exactes vérifiées : commits de tête et commits fusionnés, runs GitHub terminés en succès, marqueurs de tests et limites de chaque preuve.
- Nouveau registre `config/native-migration-evidence.json` pour les étapes Core→Rust (distinct du registre HAOS→HAOS existant), index docs/CERTIFICATION_EVIDENCE_REGISTER.md et contrat strict scripts/test_native_migration_evidence.py intégré dans CI.
- Le contrat rejette les preuves dupliquées, les marqueurs disparus, les liens non conformes et tout glissement de certification de laboratoire en production. Aucune restauration antérieure n'est rejouée.
- La production demeure NOT_PRODUCTION_READY, 0/15 ; les prochains chantiers doivent faire progresser vraie migration HAOS→Rust→HAOS, supervision durable WebSocket, intégrations universelles, services et automatismes.


## 2026-10-25 — P8w, vérification autoritative de la cohérence Core/WS/Moli

- **Anti-redondance :** les PR #89–#94 sont déjà fusionnées et valident l'identité, les états, l'abonnement WS, l'exécutable borné et le vrai Core 2026.10.2 ; les anciennes restaurations HAOS→HAOS restent acquises.
- **Correction :** une réponse get_states réussie ne prouve pas que les événements ont été reçus. Elle est désormais comparée à la ShadowSession et aux états réellement publiés par le Hub Moli. Divergence, horodatage périmé ou état manquant => invalidation globale, suppression des appareils et nouveau snapshot obligatoire.
- **Preuve Core renforcée :** 8 événements successifs attendus, vrai refus auth_invalid d'un jeton fictif erroné. CI GitHub du SHA exact nécessaire avant toute certification du scénario.
- **Tests locaux :** nouveaux tests de divergences et suppression fail-closed, aucune commande physique ; release toujours 0/15.
- **Référence :** docs/HA_CORE_WS_STATE_CONSISTENCY_2026-10-25.md.


## 2026-10-25 — P8v : vrai Core 2026.10.2 + WebSocket authentifié vers Rust/Moli

- **Sans redondance :** PR #89–#91 réelles Core→Moli (registre/états), #92–#93 WebSocket synthétique et runner CLI déjà fusionnées ; restaurations HAOS→HAOS inchangées.
- **Nouveaux scripts :** véritable Core officiel containerisé, compte utilisateur et jeton éphémères, registre et état Core réels, WebSocket Core authentifié ; second conteneur Ubuntu exécute Rust dans l'espace réseau Core – aucun port hôte ni réseau sortant. L'isolement Docker internal avec publication 127.0.0.1 a échoué au test local et a été remplacé par network none + container namespace.
- **CI réelle exigée :** workflow Core dédié + preuve P8V_GENUINE_OFFICIAL_CORE_WEBSOCKET_AUTH_TO_ACTUAL_RUST_MOLI_PASS, export et rollback redb, tests statiques de non-exposition du jeton. Ne pas annoncer de réussite avant CI SHA exact.
- **Limites :** aucun matériel réel, aucune action de service, aucun réseau de foyer, aucun HAOS productif ; 0/15 production. Voir docs/GENUINE_CORE_WS_TO_MOLI_2026-10-25.md.


## 2026-10-25 — P8u : runner WS→Moli utilisable en ligne de commande

- **Réutilise :** PR #89 registre Core, #90 Moli, #91 états Core, #92 événements WebSocket en lecture seule ; restaurations HAOS déjà certifiées inchangées.
- **Nouvelle fonction :** binaire natif avec entrée --lab-ha-ws-to-moli, durée/événements bornés, authentification WebSocket exclusivement loopback, jeton par environnement, accès aux seuls lots Core redb déjà activés, événement réel confirmé par le Hub Moli avant résultat JSON.
- **Tests :** vrai serveur WebSocket fictif authentifié, ordre souscription avant get_states, événement plus récent et roundtrip de vérification, vérification du rejet des commandes et des identifiants non enregistrés. CI et strict Clippy à vérifier au SHA exact.
- **Périmètre :** outil de laboratoire sans service continu, sans droits d'écriture, sans installation familiale ; release 0/15. Documentation : docs/HA_WS_LAB_RUNNER_2026-10-25.md.


## 2026-10-25 — P8t : événements WebSocket authentifiés en laboratoire vers Moli

- Travaux réutilisés : PR #89 registre Core persistant, #90 périphériques Moli, #91 états du Core, miroir et gestion d'époques hass-rs préexistants.
- Nouvelle convergence : changements vers véritable Hub Moli via mpsc borné, acquittement, refus des timestamps anciens et contradictoires, purge unavailable/suppressions, refus des commandes et IDs non enregistrés.
- Test révélateur : fermeture TCP silencieuse non visible immédiatement par hass-rs ; ajout d'une sonde transport get_states bornée qui déclenche la révocation des périphériques.
- Test de réseau authentifié uniquement sur WebSocket loopback fictif. Pas encore de connexion WebSocket productive au vrai Core ou aux équipements de Cloud9. CI exacte exigée pour certifier l'étape.
- Détails : docs/HA_WS_TO_MOLI_EVENT_CONVERGENCE_2026-10-25.md ; statut 0/15 production.


## 2026-10-25 — P8s, instantané des états Core dans vrai Moli OS

- **Déjà acquis, pas répété :** #89 Core→identités Rust + rollback; #90 Core→Moli Hub read-only; sauvegardes HAOS précédentes.
- **Nouvelle tranche :** Core 2026.10.2 écrit réellement deux objets State.as_dict() et Rust consomme l'instantané avec contrôle des identités Core. Le driver Moli ne publie que les états observés, ignore unknown/unavailable, refuse états hors registre et commandes.
- **Sécurité :** fichier privé O_NOFOLLOW et borné; parser strict sans clés en double; tous les points Moli restent non inscriptibles.
- **Preuve réelle :** P8S_REAL_CORE_STATES_OBSERVED_IN_NATIVE_MOLI_READONLY_PASS doit réussir en CI au SHA exact avant toute attestation de l'essai end-to-end.
- **Limites :** aucun abonnement WS continu, aucune automatisation, aucune commande d'appareil, aucune clé privée. Release NOT_PRODUCTION_READY 0/15.
- **Doc :** docs/HA_CORE_STATES_TO_MOLI_SNAPSHOT_2026-10-25.md.


## 2026-10-25 — P8r, entités Core mappées dans le vrai Hub Moli/Rust

- **Pré-existants :** PR #89 export de vrais identifiants Core→redb Rust, et restauration HAOS→HAOS déjà acquise. Aucun replay de ces certifications.
- **Nouvelle fonction :** module native-kernel/ha_registry_shadow, driver Moli sans aucune écriture, points à état inconnu, contrôle anti-collision, vérification de la propriété d'identité et refus réel des commandes Moli.
- **Tests :** 2 entrées puis 1 232 entrées de divers fournisseurs dans un véritable Hub Moli, refus de toute écriture. Validation réelle dans le conteneur Home Assistant Core attendue de la CI GitHub.
- **Prochain manque :** flux des états Core et automatismes vers la machine d'états Moli ; aucune validation production, release reste 0/15.
- **Documentation :** docs/NATIVE_MOLI_HA_REGISTRY_SHADOW_2026-10-25.md.


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

## Immutable verified P8ag–P8al Rust host evidence (26 Oct 2026)

- P8ag PR #108: 14/14 exact feature-head GitHub CI, run 40250349459; long-lived Rust/QMP lifecycle.
- P8ah PR #109: 15/15 exact feature-head GitHub CI, run 40251693506; fixed 127.0.0.1 UI forwarding.
- P8ai PR #110: 16/16 exact feature-head GitHub CI, run 40252930987; real official Core HTTP frontend.
- P8aj PR #111: 10/10 exact feature-head GitHub CI, run 40254861417; real systemd two HAOS/QEMU process lifecycles.
- P8ak PR #112: 11/11 exact feature-head GitHub CI, run 40255870443; reproducible Debian guest package real dpkg lifecycle.
- P8al PR #113: 9/9 exact feature-head GitHub CI, run 40257162323; offline signed APT index and tamper refusal.

All six are MERGED, latest main a756696b60081d641ad7ec9cf43144f3361a201b. Do NOT rerun as newly unproved work. Machine-readable source config/haos-rust-guest-evidence.json keeps each exact original feature SHA, merge SHA and workflow marker source. P8ag–P8aj official HAOS 18.3 QEMU/KVM is an isolated genuine guest; P8ak real Ubuntu dpkg and P8al cryptographic offline APT use disposable CI runners. No household deployment, no signed production trust key, no actual App service parity, no physical radios, no whole-host backwards cutover or endurance qualification. Production remains **0/15** fully certified.


### P8am live CI diagnostic and source-license bookkeeping (2026-10-26)

- At first PR #115 head 304c848, official HAOS systemd Type=notify test timed out before READY. Root cause: the new QMP detection used literal escaped quote bytes instead of normal JSON quote bytes. Readiness now calls the SAME QMP greeting parser as the previously certified shutdown path; new regression fixture rejects double escaping, Rust 16/16 and strict Clippy pass. Re-run CI on new exact head; do not claim systemd READY verified before it passes.
- A newly pinned native crate sd-notify 0.5.0 increases Cargo.lock to 227 resolved packages, 215 registry dependencies, 219 external and 213 with bundled license/notice source. Regenerated unsigned SPDX 2.3 source graph (227 packages, 526 edges) and updated six historical upstream license receipts ONLY to the new Cargo.lock SHA. Legal redistribution/signed product compliance are still NOT certified.

## 2026-10-26 — P8an fail-closed systemd recovery after genuine QEMU SIGKILL (CI pending)

P8am #115 successfully certified Type=notify/watchdog 26/26 CI, merged 01265a99a8548274fd454fdc81b5037ff299c60e. Separate P8an branch does not rerun these as new evidence. New opt-in fake-guest-only SIGKILL mode in existing real official HAOS systemd QEMU fixture; watches native systemd Restart=on-failure, distinct Rust MainPID, same persistent QCOW2 inode, no orphan after recovery stop and original backing hash. New pinned KVM workflow p8an-crash-restart.yml required at exact SHA. NO family host changes, 0/15 production.

P8ao official Podman 4.9.3 real generator rejected [Container] Memory (newer Podman key). Moved memory budget to native systemd [Service] MemoryMax (supported on Ubuntu 24.04) while preserving PidsLimit, capabilities and read-only root; exact-head CI rerun required.

P8ao runner with Ubuntu Podman 4.9 successfully generated all container and volume units. Final CI assertion assumed '--network none' and '--cap-drop all', but the genuine generator uses '--network=none' and '--cap-drop=all'. Assertions corrected to exact emitted flags and expanded to include NoNewPrivileges, MemoryMax and Pull=never; rerun pending.

- First P8aq GitHub run 40289862772 failed shortly after Redis writer quiesced/Podman export, before checksum marker. Podman 4.9 may create volume export output with its own file permissions; explicitly chmod 0600 after export and emit unambiguous archive/Redis RDB markers. Rerun exact head required, not a certified restore yet.


## Final merged proof ledger P8am–P8as — 26 October 2026

- P8am PR #115: 26/26 checks green at original feature head, GitHub Actions run 40261132205; real systemd QMP READY and watchdog on official HAOS KVM.
- P8an PR #116: 11/11 checks green at original feature head, GitHub Actions run 40263269849; real QEMU SIGKILL and automatic systemd service recovery on same HAOS disk.
- P8ao PR #117: 20/20 checks green at original feature head, GitHub Actions run 40264974609; native Podman Quadlet generator parses two generic digest-pinned OCI Apps.
- P8ap PR #118: 21/21 checks green at original feature head, GitHub Actions run 40266019276; real rootless Redis OCI App two lifecycle rounds and persistent data.
- P8aq PR #120: 11/11 checks green at original feature head, GitHub Actions run 40290122155; real Redis volume backup SHA and tampered archive refusal, separate restore.
- P8ar PR #121: 23/23 checks green at original feature head, GitHub Actions run 40291069681; Rust native manager installs, starts, stops and removes App but retains data.
- P8as PR #122: 24/24 checks green at original feature head, GitHub Actions run 40292676656; explicit 127.0.0.1 rootless Podman TCP with authenticated Redis PING.

These seven PRs are MERGED, latest base main 95123e09ac73d63da28e6106fb63794cc2fa85af.
There are now TWO separate machine-readable evidence ledgers:
config/haos-rust-guest-evidence.json (12 host/packaging cases (10 genuine HAOS/KVM and 2 real Ubuntu DEB/APT cases)
P8ac–P8an, excluding p8ao onward) and config/native-oci-app-evidence.json
(5 genuinely running rootless Podman/Redis cases P8ao–P8as). Original PR
head SHA, GitHub workflow run, merged commit, distinct real marker,
and exact CI count are pinned and checked on every PR. Do NOT redo
previous passed stages merely because a later stage is incomplete.

P8as 127.0.0.1-only host INGRESS does not prohibit container outbound
egress. A real Redis test does not imply general Home Assistant Supervisor
App/HACS parity, real physical radios, operator-authenticated production
writes, security/availability endurance, secure signed distribution or a
complete HAOS→Rust→HAOS household cutover. Product 0/15 fully certified;
no family HAOS system modified or rebooted.

- P8at PR #124 is now MERGED after all 25/25 CI, commit 42db2dc1f1bf53873b37a95e83b88c6dfd5bd658. P8au is rebased on that exact main and preserves previous evidence. P8au opted native HealthOnFailure=kill (Podman Quadlet 4.9) for subsequent asynchronous unhealthy states to request systemd restart; the bounded Rust upgrade health loop is still responsible only for immediate image rollback, not infinite-uptime rollback.

- First real P8aw CI run 40301156020 verified native Registry 3.1.2 startup, no-autostart, proper /var/lib/registry Podman mount and 127.0.0.1-only port; actual PUT blob returned HTTP 400 because Python test client omitted REQUIRED Content-Type: application/octet-stream and defaulted to urlencoded. Added conformant header per OCI Distribution API and constrained local-only response-code diagnostics. Rerun exact SHA required; not yet certified persistent HTTP data.

## 26 October 2026 — no-retest per-gate production matrix

Compared main b1881a6 against `config/release-gates.json`, the independent-signature attestation requirements, and the immutable evidence registers. Added optional machine-readable 15-item audit to existing release guard (not another certification system), plus negative regression tests. Existing 9 partial / 6 blocked / 0 certified status remains unchanged. Real HAOS same-VM and second-VM restores and OCI Redis/Registry proof not rerun. No production host, radios, secrets or backups modified.

## 26 October 2026 — P8ay race containment, NOT production certification

New issue independent of previously pinned Redis/Registry results: install, start and stop lacked per-App flock although upgrade/rollback/remove used it. Hardened existing native Rust OCI manager (no custom watcher/services) by locking before install descriptor creation and throughout start/stop; do not unlink the lock on remove. Added genuine inter-process denial and removal/reinstallation inode-continuity assertions to existing real Redis workflow. No radio, host change, new separate test infrastructure, or household deployment. Existing P8ar-P8aw proof remains unchanged; exact CI SHA must pass.

## 26 October 2026 — P8az real OCI credential delivery (new feature)

Upstream Podman Quadlet offers native `Secret=`. Added bounded non-privileged secret reference by name with pre-existing native store check. No custom secret daemon, disk copy, env credential or sample production secret. Isolated independent rootless Redis SHA256 mount CI tests correct data + negative/missing secret and cleanup. Limits: explicit lab flag; not full credential lifecycle/RBAC/HAOS migrations, release 0/15.

## 26 October 2026 — P8at–P8az immutable independent OCI proof reconciliation

These six distinct cases were already completed in real, disposable GitHub CI and merged. Their exact SHA/run/marker is now pinned in `config/native-oci-app-evidence.json`; **do not rerun** unless a material version/code/environment change invalidates a specific test. All cases are lab only, no household or genuine production secret was used.

| Case | PR | Original head | Merge | Run ID | Checks | Actual new scoped behavior |
|---|---:|---|---|---:|---:|---|
| P8AT | #124 | `f11a8022` | `42db2dc1` | 40295014593 | 25/25 | generic oci sha256 image upgrade and explicit rollback |
| P8AU | #125 | `fa64f294` | `05008d4c` | 40297108559 | 26/26 | real redis native oci healthcheck automatic incompatible image rollback |
| P8AV | #126 | `a4f66d11` | `6ed03afa` | 40299977840 | 10/10 | simultaneous distinct rootless redis and memcached generic lifecycle |
| P8AW | #127 | `be76a29a` | `b1881a64` | 40301611933 | 28/28 | stateful docker distribution registry custom oci data volume http |
| P8AY | #130 | `3004210c` | `075e18e5` | 40314671475 | 18/18 | real rootless oci lifecycle lock install start remove reinstall |
| P8AZ | #131 | `ddcd3550` | `a351529c` | 40315761886 | 20/20 | native podman secret reference real rootless redis sha256 mount |

The existing five P8ao–P8as immutable proofs remain intact. The test suite verifies all eleven cases and their exact original sources; production still has 0/15 accepted independent gates.

## 26 October 2026 — progressive independent release certification contract

Discovered an accidental forever-0/15 condition in five CI scripts: they rejected all future legitimate gate certifications, even though the actual release checker supported double-signed Ed25519 scope-specific per-criterion proof. Changed CI tests to derive counts and accepted gates from the real checker, while retaining historical 24 October planning and HAOS laboratory snapshot invariants; unsigned/tampered/fake certifications still fail. Verification: local gate, attestation, evidence truth, public ledger, masterplan and historical QEMU evidence scripts PASS; production signoff and deployment remain false. Current release remains 0/15.

## 2026-10-26 P8bb supplement

Located four more secondary test assertions coupled to current literal 0/15. Replaced status assertions with release gate audit, preserving frozen original data in source-licence, fuzz, APT and QEMU escrow evidence. Relevant direct tests and targeted unittest variants PASSED. An obsolete Codespaces test references absent original files and was intentionally left unchanged rather than adding fictional replacements. No deployment, 0/15 release.

## 26 October 2026 — P8bd latest official Core compatibility CI candidate

Official upstream Core stable 2026.10.4 dated 24 October (GHCR manifest digest sha256:23f4dfbb69ce76315dbc4de7915504f1c2dcecd7ca2648113a493a385972e133). Added independent workflow using same real strict isolated config-entry lifecycle/negative/no-egress harness, actual version assertion and separate pin, keeping 2026.10.2 and 2026.11-dev originals untouched. CI pending; current 0/15 release, no family host.

P8bd first GH run 40318906283 real Core 2026.10.4: FAIL at `/hars-lifecycle.py` line 22 before API testing due new test harness import from nonexistent top-level `homeassistant.__version__`. Matched the existing tested official `homeassistant.const.__version__` import in `scripts/test_stable_core_no_marketplace.py` and upstream Core versioning docs; exact new-head CI mandatory, no false claim of product incompatibility.

## 26 October — canonical handoff reconciliation after final Core 2026.10.4 CI

GitHub PR #134 final exact head 21d9b21421a215856930764f2264509f06459a3a and run 40319204128 SUCCESS, 10/10 checks; merged as 16a1f49b9b17e2175e30268cd4fc545b2121a05f. LXC700 canonical product working tree fast-forwarded to main 16a1f49 and two disposable session QA worktrees removed; other concurrent/historical worktrees untouched. Seven recent PRs #128-#134 merged after independent checks, retained exact original product and laboratory constraints. Current gate checker still 9 partial, 6 blocked, 0/15 certified, no production signoff or deployment. Canonical handoff NOW begins with verified actual state and explicitly marks older notes historical; next nonredundant development starts from that summary.

## 26 October 2026 — P8be source custody (NOT automation parity)

Preserve automations.yaml as a byte-exact, bounded, unexecuted, archive- and entity-registry-bound input in the existing private redb store. The immutable archive custody and real HAOS P8aa import remain separate. New original material remains inactive; raw YAML is never logged nor interpreted or targeted at home radios. Tests cover durable reopen, replay idempotency, rebind refusal, dual legacy/modern archive path refusal, UTF-8/size limits, rollback of identity stage leaving original proof intact. A full automation migration/execution, secrets, platform-specific custom components and reverse full-stack conversion remain NOT VERIFIED. Current product release 0/15.

## 26 October P8bf automation restore fidelity

Extended existing real HAOS 18.3 QEMU two-guest source/target fixtures with one fictitious disabled automation. Validate original SHA256 on source and after independent official Supervisor full restore to different guest. P8p verifies direct transport; P8ab verifies native Rust escrow + restored target. New marker per path with no new services or third CI harness. Neither executes automations or establishes Rust automation compatibility. Independent CI pending, 0/15.

## 26 October 2026 — immutable P8be/P8bf lab evidence indexed

P8be PR #136 merged 004f2cb, exact feature HEAD 3527be5, 12/12 green, real Supervisor HAOS run 40328174220, original automation YAML opaque inactive native redb. P8bf PR #137 merged 18b519e, exact HEAD ed07cc7, 12/12 green, genuine separate fresh-HAOS direct restore run 40330463271 plus original archive through native Rust escrow fresh HAOS restore run 40330463267, byte-exact disabled fixture SHA after restoration. The three distinct cases are now appended to existing machine-proof register with source markers; no duplicate register or environment. Current product remains release NOT_PRODUCTION_READY, 9 lab partial/6 blocked/0 independently certified; no actual home VM100, app state or devices touched.

## 26 October 2026 P8bh actual Core WS recovery (ongoing)
New distinct scenario added to existing official Core (p8v) CI: create authenticated read-only WS/Moli shadow, explicitly revoke/cancel old Moli Hub, reject stale reads and invalid-token login on genuine Core, then reauthenticate a fresh epoch, subscribe-before-snapshot and verify state published by a NEW native Hub, revoke again. No Core service calls, no physical devices, no real HAOS token. This is not multi-day reconnect, nor field certification; CI must pass before merging.

## 26 Oct 2026 — P15 archive importer real libFuzzer target in CI

New fifth sanitizer target invokes actual bounded Rust HAOS unprotected tar importer, tests validated original registries and inert automations with five entirely fictional tar seeds (clean, protected, duplicate alias, traversal, malformed UTF-8). Extends existing nightly-2026-10-15 libFuzzer/ASan workflow, 45s PR smoke and 180s scheduled, without touching household or uploading real archives. The root Cargo.lock is unchanged; separate fuzz/Cargo.lock updated. Local compile and corpus checks PASS; awaited GitHub sanitizer CI must be cited before claiming real fuzz pass. Release remains blocked.

## 26 October 2026 — P8bh/P15 exact pinned independent lab receipts (0/15 production)
PR #139 native Core WS authenticated generation recovery, head 83b90ab, merge 2828751, GH real Core run 40340882977, 9/9 CI. PR #140 HAOS full archive import real sanitizer fuzz head 8453d35, merge 016c002, GH run 40342730820, 11/11, 263654 iterations/46s. New fifth fuzz source and immutable fake corpus. Extended five-target 180s GH run 40343956190 started, conclusion pending. Historical proofs kept, no second redundant harness or field machine touched. All production gates stay blocked.

## 26 October — Tokio patch + source/SBOM provenance unblock (independent branch)

Fixed unrelated blocked Dependabot #119 by preserving, not bypassing, its two fail-closed SHA-bound license provenance tests. Tokio 1.54.1 updated Cargo.lock only, and generated SPDX full 227-package graph plus the already pinned six upstream license evidence records were regenerated against exact Cargo.lock bytes. No new external copyright claims; legal compliance and production release still FALSE. Local complete workspace tests, Clippy, public redaction and source license contracts green. Full CI pending on separate branch, concurrent HAOS #142 untouched.
## 26 October — P8bi genuine HAOS archive to native executable fake Moli graph (CI pending)

New scoped chain covers one explicit **disabled** fictional Home Assistant YAML automation from a genuine disposable Supervisor full archive, byte-exact private redb custody, validated matching original entity-registry identities, safe YAML subset compile, upstream Moli automation engine save disabled and explicit in-process lab approval, synthetic device actuation, approval invalidation after edit, rollback refusing further execution. All other automations (templates, multiple triggers, time, complex actions, unsupported domains, real devices) fail closed. Local synthetic archive and 53 Rust tests/clippy green. Official genuine HAOS QEMU + native engine pending CI; neither family VM100 nor devices touched. Still NOT_PRODUCTION_READY; no automation data from a household tested.

## 26 October — P15 disabled HAOS source and native Moli compiler fuzzing (pending)

New sixth sanitizer-instrumented independent Rust libFuzzer target tests the exact real P8bi safe disabled-only Home Assistant YAML source list parser plus native Moli graph compiler. Includes six synthetic corpora: one valid disabled, enabled refused, template refused, nonlab entity refused, duplicate key refused, two automations refused. Limits to 8192 bytes and never starts native Hub/HAOS/driver, no radio, no token or real device. Reuses existing pinned-nightly GitHub fuzz workflow, 45-second PR smoke and 180-second scheduled/dispatch extended proof, separate fuzz Cargo.lock, source root Cargo.lock and source SBOM unchanged. Target is one parsing/compiling sub-surface, not general YAML/HAOS acceptance or independent pentest. GitHub CI pending.
## 26 octobre 2026 — P8bj édition sauvegardée, puis redémarrage (CT700)

P8bi #142 fusionnée (13/13 checks au HEAD 2293d85, merge 6eeda5b). Lacune réelle : prove_plan vérifiait changed.is_approved() sur copie non sauvegardée. Nouveau test upstream Moli : édition Agent persistée, ancienne empreinte refusée, zéro run/action après édition, arrêt/recréation du Hub et moteur, refus et zéro run/action maintenus. Preuves locales : 3 essais Rust ciblés PASS (0,80 s chacun), 2/2 tests unitaires historiques ciblés PASS, binaire natif PASS, Clippy strict PASS, fmt/diff PASS. Branche P8bj poussée après rétablissement de l'authentification GitHub CT700; PR et CI exactes à vérifier. Aucun appareil réel; 0/15 production.

## 26 octobre 2026 — P8bi #142 et P8bj #144 fusionnées, preuves CI exactes

#142 head 2293d85b0d2c0c1ef514e2be75ec4f25f48b75f9, merge 6eeda5b718f3bd6b214053808e082841bd7494b9, 13/13 GitHub SUCCESS ; QEMU HAOS véritable 40348951772. #144 head 06e6a977a2c45cea3d04ec124241c552d90dee8e, merge 0e16eac7f848d98fc0c16b5ce80b9a7c371f8faf, 10/10 GitHub SUCCESS ; native Rust 40352628593, vraie régression QEMU HAOS 40352628694. P8bj nouveau comportement réel : agent modifie et sauvegarde dans Moli, l'ancienne approbation est refusée, aucun nouveau run/action, et la révocation survit à la reconstruction engine/Hub. Preuve de laboratoire seulement, acteur humain fictif, 0/15 certifications. GitHub fuzz prolongé P15 40343956190 a également terminé SUCCESS au SHA 016c002. Main CT700 synchronisé à 0e16eac avant le présent correctif documentaire.
