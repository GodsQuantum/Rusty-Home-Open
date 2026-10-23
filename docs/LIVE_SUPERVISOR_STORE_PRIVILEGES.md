# Generic, fail-closed pre-install Supervisor App permissions review

For every valid third-party Supervisor Store slug, Rusty Home fetches
the official Store App detail with the native Supervisor token, after verifying
current exact catalog membership and a unique live Supervisor backend.
The typed parser extracts only bounded official security metadata:
nine known boolean capability fields (missing means UNKNOWN, not false),
official integer security rating (1 to 8), and allowlisted Supervisor
role and AppArmor mode.

The returned JSON never contains configured options, secret values,
device paths, image locations or arbitrary third-party strings.
High privileges or low rating are displayed as elevated review.
Unknown data cannot be classified safe. No outcome authorizes an install:
read_only=true and activation_authorized=false.

The dashboard fetches the review only when the user selects
Sécurité et privilèges, rendering with textContent.
This does not yet validate hardware permissions, third-party publisher
authenticity, backup/rollback prerequisites or authorize lifecycle POSTs.
It is a product UX/security increment, not production certification.

Supervisor reference:
https://contributors.home-assistant.io/apps/supervisor-api/store/
