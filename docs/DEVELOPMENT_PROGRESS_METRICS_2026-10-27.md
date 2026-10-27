# Rusty Home — indicateurs d’avancement reproductibles (27 octobre 2026)

**Le chiffre de 60 % répété mesure UNIQUEMENT la couverture partielle des critères officiels, pas le développement terminé.** Il ne bouge pas lorsque plusieurs preuves plus approfondies sont ajoutées dans un critère déjà couvert. Pour mesurer réellement le travail, trois autres indicateurs distincts sont nécessaires.

## Indicateurs conservateurs et calculables

| Mesure | Valeur actuelle | Interprétation exacte |
| --- | --- | --- |
| Couverture des critères officiels | **9/15 = 60 %** | Critères marqués lab_partial dans config/release-gates.json |
| Certification de production | **0/15 = 0 %** | Critères intégralement certifiés ; déploiement toujours interdit |
| Indice conventionnel de maturité technique | **25/60 = 41,7 %** | Somme de 15 × quatre niveaux à poids égal, étayés par des tests ; **pas** la fraction réelle du produit développée |
| Parcours actif migration des applications HAOS | **4/10 = 40 %** | Dix jalons fixes, chaque jalon terminé a un ID de preuve CI et un marqueur de test |
| Répertoire des preuves laboratoire | **47** | Exécutions de test distinctes déclarées, jamais une mesure de pourcentage |

L'indice 41,7 % est **une convention de suivi**, pas une approximation fiable de la charge restante ni une prédiction de date. L’échelle : 0 = pas de prototype, 1 = brique démontrée de façon préliminaire, 2 = cas réel validé en CI isolée, 3 = fonctionnalité représentative de bout en bout intégrant panne et récupération, 4 = critère complètement qualifié et signé indépendamment. L’indice compte 15 critères à poids égal : l’effort d’un pilote radio universel et celui d’une notice de licence ne sont PAS équivalents. Ne jamais présenter 41,7 % comme un ETA ou comme la progression réelle de l'objectif ambitieux « Core intégralement Rust ».

## Parcours actif — sauvegarde Supervisor vers applications natives

| Jalon | État | Preuve vérifiée |
| --- | --- | --- |
| 1. Sauvegarde réelle App-only par Supervisor | Terminé | PR #206, HAOS 18.3 officiel |
| 2. Archive en coffre privé Rust avec SHA-256 | Terminé | PR #210, copie byte-exact |
| 3. Données restaurées en fichiers privés inactifs | Terminé | PR #213, vrais octets File Editor |
| 4. Échec avant publication : aucun fichier partiel | Terminé | PR #213, injection de panne et rollback |
| 5. Démarrage d'une image App officielle sur ces données natives | Ouvert | Non prouvé |
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
