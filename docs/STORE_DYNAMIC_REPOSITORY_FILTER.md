# Dynamic official Supervisor Store repository selection

Product scope: read-only UI over the **real authenticated** Supervisor Store
API already implemented in Rust. The dashboard consumes the catalogue's
repositories and apps fields and builds the dropdown from the current
official response. This works for arbitrary future vendors and repositories.

- Exact repository slug filter combines with case-insensitive name/slug/repo
  search and existing 24-per-page navigation. No fixed App inventory.
- Repository names are inserted only as DOM text, never HTML. Identifiers,
  duplicates, control characters and 2048-entry upper bound are validated.
- A failed or ambiguous Store refresh removes stale cards, clears the selector
  and disables both the repository filter and search.
- No write method, Supervisor POST, user credential, personal configuration,
  radio or HAOS production VM is accessed.
- The browser integration scenario checks 125 unknown Apps across 17 repos,
  combined search, injection-like text and malformed duplicate repo denial.
- Local tests: Python UI contract, Node execution of the actual browser
  filter/validator over 1317 fictional Apps and 23 repositories, syntax checks,
  full Rust workspace and strict Clippy. CI results must be recorded against
  the PR SHA after independent GitHub Actions complete.

This is **not** trusted operation RBAC, persistent receipts, App mutation,
migration or production certification. The 15 release gates are unchanged.

Local environment limitation: the Playwright 1.57 browser installer does not
support Ubuntu 26.04 in DEV. Browser-level execution is therefore delegated
to the independent Ubuntu 24.04 GitHub Actions runner. The Node algorithm
tests and source-contract checks passed locally; this limitation is not a
browser PASS.

Feature implementation commit: 371c0a65d1005274b47c9704084791b0bb615c3c; PR #51.
