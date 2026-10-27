# Rusty Home — indicateurs d’avancement reproductibles (27 octobre 2026)

**Le chiffre de 60 % répété mesure UNIQUEMENT la couverture partielle des critères officiels, pas le développement terminé.** Il ne bouge pas lorsque plusieurs preuves plus approfondies sont ajoutées dans un critère déjà couvert. Pour mesurer réellement le travail, trois autres indicateurs distincts sont nécessaires.

## Indicateurs conservateurs et calculables

| Mesure | Valeur actuelle | Interprétation exacte |
| --- | --- | --- |
| Couverture des critères officiels | **9/15 = 60 %** | Critères marqués lab_partial dans config/release-gates.json |
| Certification de production | **0/15 = 0 %** | Critères intégralement certifiés ; déploiement toujours interdit |
| Indice conventionnel de maturité technique | **25/60 = 41,7 %** | Somme de 15 × quatre niveaux à poids égal, étayés par des tests ; **pas** la fraction réelle du produit développée |
| Parcours actif migration des applications HAOS | **5/10 = 50 %** | Dix jalons fixes, chaque jalon terminé a un ID de preuve CI et un marqueur de test |
| Répertoire des preuves laboratoire | **48** | Exécutions de test distinctes déclarées, jamais une mesure de pourcentage |

L'indice 41,7 % est **une convention de suivi**, pas une approximation fiable de la charge restante ni une prédiction de date. L’échelle : 0 = pas de prototype, 1 = brique démontrée de façon préliminaire, 2 = cas réel validé en CI isolée, 3 = fonctionnalité représentative de bout en bout intégrant panne et récupération, 4 = critère complètement qualifié et signé indépendamment. L’indice compte 15 critères à poids égal : l’effort d’un pilote radio universel et celui d’une notice de licence ne sont PAS équivalents. Ne jamais présenter 41,7 % comme un ETA ou comme la progression réelle de l'objectif ambitieux « Core intégralement Rust ».

## Parcours actif — sauvegarde Supervisor vers applications natives

| Jalon | État | Preuve vérifiée |
| --- | --- | --- |
| 1. Sauvegarde réelle App-only par Supervisor | Terminé | PR #206, HAOS 18.3 officiel |
| 2. Archive en coffre privé Rust avec SHA-256 | Terminé | PR #210, copie byte-exact |
| 3. Données restaurées en fichiers privés inactifs | Terminé | PR #213, vrais octets File Editor |
| 4. Échec avant publication : aucun fichier partiel | Terminé | PR #213, injection de panne et rollback |
| 5. Démarrage de l'image officielle File Editor avec données restaurées | **Terminé** | PR #218, vraie image OCI rootless, marqueur exact visible dans le conteneur |
| 6. Utilisation fonctionnelle sous authentification/ingress isolé | Ouvert | Non prouvé |
| 7. Données natives conservées après redémarrage App | Ouvert | Non prouvé |
| 8. Upgrade/downgrade App et rollback des données | Ouvert | Non prouvé |
| 9. Deuxième App officielle différente, données utilisables nativement | Ouvert | Non prouvé |
| 10. Chaîne complète multi-App et rollback sur faute | Ouvert | Non prouvé |

**Évolution visible :** avant PR #213, ce parcours était à 2/10 ; après ses deux jalons indépendants, il est à 4/10, soit **+20 points de progression sur ce sous-projet**. Cela était invisible dans les 60 % de couverture des critères. Les dix jalons ne prétendent ni remplacer Home Assistant Core, ni certifier neuf Apps, ni autoriser le foyer.

## Contrôles automatiques

La définition, l'ordre et la provenance des jalons sont fixés dans config/development-progress.json. Le script scripts/check_development_progress.py calcule tous les pourcentages à partir de ces fichiers et de config/release-gates.json et config/lab-evidence.json. scripts/test_development_progress.py doit refuser un jalon coché sans source, un faux ID de CI, une preuve d'un autre gate, une promotion de maturité injustifiée et le saut d'un jalon manquant.

Commande : python3 scripts/check_development_progress.py --json && python3 scripts/test_development_progress.py

La progression est suivie pour la **voie B** (gestionnaire Rust, compatibilité conservée avec Home Assistant Core/HACS officiels), définie dans le plan de certification. La **voie C** (remplacer aussi tout le vrai Core et ses intégrations par un moteur natif Rust) possède une incertitude ouverte ; aucun pourcentage global factuel ne peut être calculé pour ce chantier aujourd'hui.

## Jalons P8df — 27 octobre 2026

Depuis PR #218, le sous-parcours App HAOS passe de 4/10 à **5/10, soit 50 % de ses dix jalons fixes**. Le test HAOS réel 18.3 (GitHub https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40541956380, job 122322394014, JUnit 3/3) a réellement démarré l'image officielle ghcr.io/home-assistant/app-configurator:6.3.0 à partir des fichiers de /data restaurés par Rust, en conteneur Podman rootless sans réseau ni ports. Marqueur source vérifié en direct dans le processus et archive SHA inchangée. PR #218 fusionnée, source 0682b88c78fb5c03fd72bedb5b4e01f5db00d4a5, main f2e27f5c0a75451fa44664b5f7f42d9893a66e53, 10/10 checks.

Le **jalon 6 (fonctionnement UI authentifié avec Ingress)** demeure non testé : présence des octets dans un processus démarré ne démontre pas que l'éditeur peut être utilisé. Aucun changement d'état de certification indépendante ; production 0/15, couverture 9/15, indice conventionnel 25/60.
