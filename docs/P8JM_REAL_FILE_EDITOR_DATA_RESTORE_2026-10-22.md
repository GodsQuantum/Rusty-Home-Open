# Restauration de données d'une application réelle sur HAOS — preuve P8j.m

**Statut du produit : NOT_PRODUCTION_READY.** Ce contrôle ne s'exécute que dans une **synthetic VM** : une machine Home Assistant OS 18.3/QEMU jetable de GitHub Actions. Aucune installation HAOS familiale, donnée privée, archive personnelle ou périphérique Zigbee/Matter n'est utilisé.

## Portée du test réel

Le scénario `test_hars_real_official_file_editor_partial_restore_bytes_qemu` emploie l'application officielle File Editor (`core_configurator`), et non une fausse API de sauvegarde :

1. Installation réelle via le Supervisor, vérification de son slug, de sa version et de son état.
2. Identification du véritable répertoire de données géré par le Supervisor. Les chemins `apps/data` et `addons/data` sont détectés sans inventer un emplacement supplémentaire.
3. Création de **données fictives** dans ce répertoire de test seulement, puis mesure SHA-256 des octets.
4. Création d'une vraie sauvegarde partielle officielle de cette application, contrôle des métadonnées et SHA-256 de son archive.
5. Modification effective des octets dans l'application avant d'envoyer la demande de restauration officielle.
6. Restauration partielle, puis vérification du retour exact du SHA-256 d'origine, des octets, de la version, de l'état installé, de la santé Core/Supervisor et de l'intégrité immuable de l'archive.
7. Désinstallation et nettoyage de l'application installée par le test, uniquement dans la VM jetable.

La campagne HAOS sur <https://github.com/GodsQuantum/Rusty-Home-Open/actions/runs/40127707580> a validé **17 scénarios sur 17**, dont ce test. La présence du marqueur `P8JM_REAL_OFFICIAL_FILE_EDITOR_DATA_BYTES_RESTORED_PASS` est contrôlée par le mécanisme de qualification du workflow.

## Ce que cela ne certifie pas

Une restauration réussie de File Editor, avec données fictives, ne certifie PAS les **Apps non testées**, les données chiffrées de toutes les applications, les bases externes, les appareils réels, les restaurations d'archives d'un foyer, la récupération sur une nouvelle machine ou une migration globale sans perte.

Le décompte de 17 désigne uniquement les scénarios HAOS de ce workflow, pas un nombre maximal d'applications que Rusty Home saurait gérer. L'exigence reste une qualification générique, avec des campagnes indépendantes de restauration et de rollback.

**0/15 critères de sortie de production certifiés**, `deployment_authorized=false`. Aucune restauration à domicile n'est autorisée par cette preuve.
