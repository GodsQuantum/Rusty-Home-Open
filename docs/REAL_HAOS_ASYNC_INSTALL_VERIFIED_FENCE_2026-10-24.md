# Official HAOS native asynchronous install-to-fence verification

This proof is designed to run only in the genuine, pinned HAOS 18.3 KVM snapshot on GitHub Actions. It extends the existing 17-real-case Supervisor QEMU runner rather than shipping a new product App or touching a household system.

## End-to-end evidence

- Build a static musl Rust example from the same supervisor-apps library as the product's experimental capabilities.
- Start the already-authorized, protected manager-role Rust QEMU fixture with a real Supervisor-provisioned token and owner-private 0700 audit directory.
- Publish one independent, harmless local App fixture to the genuine official Supervisor Store.
- Construct the generic background Install request; record the lab actor, App-scoped consent and synchronous audit; durably arm its own redb dispatch before issuing exactly one official authenticated Supervisor background POST.
- Require Supervisor's returned job ID, an actual independent GET /jobs/{id} and a successful GET of the installed App inventory; no cache-derived fake completion.
- Persist a confirmed operation, remove only its App lock with a two-phase redb commit, reopen the ledger and verify the terminal record survives.
- Print one stable, nonsecret P8N success marker, then uninstall and delete ONLY the explicitly created disposable fixture.

The actual QEMU test fails if Rust does not complete each step, the real binary is not statically linked, the guest Store is unavailable, the postcondition fails, or a binary mutation is attempted outside the dedicated manager fixture.

This DOES NOT prove a production end-user Core identity provider, CSRF/origin policies, encrypted backup/restore of arbitrary third-party Apps, live physical Zigbee/Matter/Thread handoff, long-run soak or audited release signatures. Default Rusty Home remains read-only. A successful CI run is additional bounded laboratory evidence and DOES NOT certify any of the 15 production gates.
