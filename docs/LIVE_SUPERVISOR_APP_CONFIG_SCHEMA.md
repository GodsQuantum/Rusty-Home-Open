# Universal Supervisor App configuration form discovery

Rusty Home now obtains the option *schema structure* for an installed App via
the authenticated official Supervisor GET /addons/{slug}/info (or opt-in V2).
There is no fixed vendor allowlist. The source must be uniquely selected,
the App must be present in the current authenticated installed inventory,
and the path and response slug must match exactly.

The typed Rust parser returns at most 128 fields, with bounded ASCII names,
an allowlisted field kind, required/optional flag and sensitive indicator.
No configured options, defaults, passwords, secret references, URLs,
device paths or custom validators are sent to the frontend. Supervisor
response content is parsed with strict duplicate-key rejection.

The dashboard shows these fields on demand using textContent only.
The HTTP endpoint GET /api/apps/{slug}/config-schema is read-only; no
configuration editing, activation or lifecycle mutations are enabled.
Unknown schema formats, unavailable Supervisor data and ambiguous App sources
fail closed. A schema=false App remains visible without an editable form.

This is a real generic product discovery feature, not proof of safe editing,
RBAC for mutations, complete integration compatibility, or 15/15 production
certification. The external official API reference is
https://contributors.home-assistant.io/apps/supervisor-api/apps/
