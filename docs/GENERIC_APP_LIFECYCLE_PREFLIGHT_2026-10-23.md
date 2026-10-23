# Rusty Home — préconditions des opérations Supervisor

Ce moteur Rust lit exclusivement les observations actuelles fournies par
le Store officiel, les détails publics de l'App et l'inventaire officiel
des applications installées. Aucun App ID n'est codé en dur : la
certification inclut **1 017 noms arbitraires** de fournisseurs.

Les sept types d'action pris en compte : installation, démarrage, arrêt,
redémarrage, mise à jour, sauvegarde et restauration. Les observateurs
confrontent l'existence de l'App, la version API V1/V2, le dépôt, l'état,
l'installation effective, la présence d'une mise à jour et le caractère
éventuellement géré par le système. Les sources divergentes renvoient
un état incohérent et aucune validation.

Les mises à jour et restaurations nécessitent encore de vérifier une
sauvegarde valide ainsi que sa destination. Ce précontrôle n'invente
ni sauvegarde, ni estimation de réussite, ni mécanisme de rollback.

Le seul résultat peut être un **candidat à examen** sur la base d'un
état cohérent. Le retour JSON porte systématiquement read_only=true
et activation_authorized=false : ce module ne déclenche et
n'autorise **aucune écriture**. Il n'ouvre pas non plus de route HTTP.

Avant une intégration en production, il reste à brancher une requête
officielle en temps réel, la session administrateur native, les droits
par App, une confirmation anti-rejeu, les sauvegardes et restaurations
certifiées et le suivi des jobs. Le registre des quinze critères de
production n'est pas modifié par cette preuve unitaire.
