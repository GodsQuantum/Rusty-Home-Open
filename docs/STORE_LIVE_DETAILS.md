# Live Supervisor Store App details (product increment)

The universal Rusty Home Store dashboard can now open **read-only details for
an App that is not installed**, without using the installed-App endpoint.
No App names or repository IDs are hardcoded.

- `GET /api/store/apps/{slug}/detail` is served only when exactly one
  `--supervisor-live` or `--supervisor-live-v2` source is configured.
- The native App-scoped client uses its Supervisor-issued token and the fixed
  internal `supervisor:80` service, not a browser-supplied host.
- It fetches the current official Store catalog and requires an exact slug
  match before requesting `GET /store/addons/{slug}` (V1), or
  `GET /v2/store/apps/{slug}` (V2 development flag).
- Slugs are strictly bounded ASCII; the typed response excludes App options,
  credentials, device paths, network addresses, arbitrary descriptions and
  all raw Supervisor fields. The UI uses text nodes, not HTML interpolation.
- Invalid slugs receive 400; absent live source or missing App receives 404;
  ambiguous source, denied token or unavailable API receives 503.
- No mutation endpoint, operation receipt or production authorization is added.

## Verification scope

TDD first demonstrated missing Rust APIs and route handlers at compilation.
Tests cover V1/V2 paths, invalid slugs, authenticated loopback HTTP GET,
unknown uninstalled providers, secret redaction, snapshot/ambiguous-source
rejection, and no detail fetch for an App absent from the live catalog.
The loopback mock is NOT a real HAOS end-to-end test.

## Next code increment

Implement an official Store availability preflight for any selected App,
then a trusted user permission lookup and durable operation receipt before
enabling installer/start/stop/update operations. Validate the whole workflow
on disposable HAOS in GitHub Actions. No fixed nine-App compatibility list.

Official API reference:
https://contributors.home-assistant.io/apps/supervisor-api/store/
