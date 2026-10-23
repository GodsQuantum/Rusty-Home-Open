# Rusty Home — Sessions persistantes et anti-rejeu (laboratoire)

Statut : composant Rust **désactivé dans les builds par défaut**. Les sessions
proviennent encore de fixtures et non d'une session utilisateur Home Assistant
authentifiée. Aucun droit d'écriture en production n'est octroyé.

## Garanties testées avec redb

- Au plus **1 024 sessions persistantes** simultanées. Les sessions expirées,
  devenues inactives ou incohérentes avec l'horloge sont évacuées avant
  l'admission, dans la même transaction.
- Au plus **4 096 confirmations** en attente au total, et **32 par session**.
  Les confirmations expirées à 60 secondes sont purgées avant réémission.
- La révocation d'une session efface **toutes** ses confirmations en attente
  dans la même transaction redb avec validation en deux phases.
- Le contrôle du quota, la suppression et l'insertion sont réalisés sous la
  transaction d'écriture, pour ne pas admettre des sessions concurrentes
  au-dessus des plafonds.
- Un jeton consommé, associé à une autre session ou à une autre opération,
  ou révoqué ne peut pas obtenir de deuxième autorisation, même après
  réouverture de la base. Les enregistrements conservent uniquement des
  condensats de jetons et non les secrets bruts.
- La journalisation fsync demeure une condition préalable à tout
  permis de laboratoire.

## Hors champ — ne pas autoriser un déploiement

Ces tests ne constituent ni un fournisseur d'identité, ni une protection
contre un administrateur système malveillant, ni la certification des
opérations Supervisor en production. Il manque le rattachement sécurisé
à la session Ingress actuelle, la révocation Core, le CSRF, une horloge
serveur faisant autorité, la pré-vérification et la compensation des
opérations, et les preuves de rollback et de restauration. Les 15 portes
de publication restent bloquées ou partiellement démontrées.
