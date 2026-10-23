# Rusty Home — autorisation Core / RBAC (laboratoire)

**PAS un fournisseur d'identité.** Ce document décrit un laboratoire
d’autorisation, pas une authentification d’utilisateur prête pour la production.

## Garanties démontrables

La revendication X-Remote-User-Id ne peut provenir que d’une connexion
TCP effective depuis le proxy Ingress de Supervisor (172.30.32.2). Cette
identité ne confère **aucun rôle** à elle seule.

core-user-snapshot-lab vérifie une réponse Core de type config/auth/list :
identifiant de requête corrélé, objet JSON sans clés dupliquées, états
is_active, is_owner, system_generated, groupes et fraîcheur inférieure
à 15 secondes. Une réponse de test ne constitue pas une preuve d’authenticité.
Le transport WebSocket au chemin interne supervisor/core/websocket dédié à HAOS/KVM n’est activable que sous le marqueur
explicite HARS_P8H_QEMU_ONLY=1.

core-user-rbac-lab (option de compilation **désactivée par défaut**)
croise la revendication Ingress, la réponse Core fraîche et les autorisations
spécifiques à chaque App. Un utilisateur inconnu, désactivé ou système est
refusé ; un utilisateur sans droits administrateur ne reçoit pas un droit
d’écriture ; une permission d’App absente n’est jamais implicite. Les
opérations destructrices nécessitent une confirmation que ce laboratoire
**ne délivre jamais**.

La campagne HAOS réelle doit constater une réponse utilisateur Core autorisée.
Un config/auth/list refusé est une observation négative, **pas** un test de
RBAC réussi.

## Limites et critères restants

Aucun endpoint de mutation, ni nouveau jeton, ni rôle admin Supervisor n’est
activé pour le produit. Il faut encore certifier le transport Core en contexte
réel, le rattachement authentifié des permissions au navigateur, les
révocations, l’anti-rejeu, la protection CSRF, les sessions durables,
l’isolation entre utilisateurs et la confirmation des opérations.

Les **15 critères** de production, notamment production_ui_auth, demeurent
bloqués ou partiels tant que ces garanties n’ont pas de preuve indépendante.
