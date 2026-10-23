# Rusty Home — session native de Home Assistant via Supervisor

État : preuve **en lecture seule**, pas une autorisation d'installation.

Sources officielles : [ingress.py](https://github.com/home-assistant/supervisor/blob/main/supervisor/api/ingress.py)
et [const.py](https://github.com/home-assistant/supervisor/blob/main/supervisor/const.py).

Le Supervisor réévalue le compte Home Assistant attaché à une session
Ingress, interdit au client de définir ses propres en-têtes
X-Remote-User-Id et X-Remote-User-Groups, puis fournit ses groupes
effectifs. Le propriétaire reçoit également le groupe system-admin
dans les en-têtes transmis. L'App Rusty Home configure déjà panel_admin:
true dans config.yaml, ce qui déclenche la vérification administrative
native dans Supervisor.

Le contrôleur Rust n'admet l'identité que depuis l'adresse TCP réelle
du proxy Supervisor 172.30.32.2. Il vérifie le format HTTP, l'Origin,
l'identité, le caractère unique des en-têtes, et dorénavant les groupes
séparés par virgule. Seul le groupe exact system-admin est considéré
comme indicateur d'un administrateur. Un groupe absent n'octroie
aucune identité administrateur. Les doublons et les valeurs malformées
sont rejetés.

GET /api/session est réservé au mode Ingress, répond uniquement par
des booléens de statut, sans identifiant, groupe, nom ou jeton :
authenticated, administrator, read_only et actions_authorized=false.
L'interface affiche clairement que toutes les commandes sont indisponibles.

**Limites :** cet indicateur n'est PAS un permis de modifier les Apps.
Il faut encore certifier, par requête, l'authentification actuelle,
les politiques d'autorisation spécifiques aux Apps, l'anti-rejeu, les
confirmations à usage unique, les révocations, les opérations et l'audit
durable. Les appels exécutables à l'API Supervisor restent absents du
produit. La certification HAOS KVM actuelle prouve le refus des
en-têtes falsifiés depuis un pair non approuvé, pas encore une véritable
session navigateur administrateur. Les 15 critères de production demeurent
non certifiés.
