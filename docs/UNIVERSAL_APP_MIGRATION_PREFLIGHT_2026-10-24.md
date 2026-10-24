# Vendor-neutral App migration preflight — 2026-10-24

The project must NOT be hardwired to one household's installed Apps, nor the nine synthetic examples in its repository. This change removes an unconditional `len(installed) != 9` refusal and admits any bounded supplied Supervisor App inventory and matching per-App migration policy, including empty installations.

The read-only Python assessor:

- Validates exactly matching App identifiers, preventing duplication, case/confusable path forms, traversal, unsafe separators and malformed records.
- Bounds the supplied inventory to 50,000 App records, checks to 64 per App, and evidence to 256 structured records per App.
- Preserves non-sensitive total/qualified/pending/critical counts, generated from the input rather than hard-coded App names or quantities.
- Refuses to treat zero Apps as a vacuous fully qualified migration. Even an inventory whose every entry claims `production_qualified` reports only `requires_independent_e2e_certification`, never a migration authorization.
- Never accesses the Home Assistant runtime, secrets, user backups or network; the original nine examples remain test fixtures only.

Tests include inventories of 0, 1, 2, 9, 10, 50 and 1,000 unrelated vendor slugs, all-qualified claims, missing/duplicate/malformed policy data and adversarial path attempts. Real backup restoration, reversible state migration, device controller ownership, authenticated production sessions and independent release evidence still need certification.

All **15 mandatory production gates remain noncertified**. This is a correctness/compatibility hardening step, not a claim that 1,000 integrations were executed.
