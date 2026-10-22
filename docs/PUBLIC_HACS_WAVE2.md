# Selected additional Home Assistant Core integration coverage

The publicly shipped CI runs five additional pinned, genuine HACS
integrations against official Core 2026.10.1 in disposable Linux Docker
laboratories:

- Card Builder 2.6.0 — actual published release, frontend / config flow
- Smart Announcements 0.8.3 — genuine Core config flow lifecycle
- Adaptive Lighting 1.32.0 — actual config entries, virtual switch logic
- Custom Universal Media Player 1.9.0 — tagged *release ZIP* executed
  in genuine Core rather than mismatched dev manifest
- Epson SNMP 0.1.3 — absent device must be rejected safely

Sources and release assets are verified with immutable Git SHAs and/or
SHA-256 checks. Each Core container is disposable, restricted, isolated
from household networks and has no real USB hardware or credential access.

Together with P6d Watchman, P6k Blueprint Studio and P6l CUP, the
public CI would cover eight **selected** real integration lifecycles,
only when every independent job for the exact revision executes and
passes. This cannot establish support for a private inventory of
29 integrations or any of nine actual Apps. All 15 release gates remain
blocked pending full migration and hardware evidence.
