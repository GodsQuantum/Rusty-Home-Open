#![no_main]
// Fuzz the EXACT production native Rust Supervisor Store V1/V2 parser.
// Bytes are untrusted JSON; never contact a Supervisor or use tokens.
use hars_supervisor_apps::official_store::parse_store_response;
use libfuzzer_sys::fuzz_target;
use std::collections::BTreeSet;

fuzz_target!(|wire: &[u8]| {
    if let Ok(catalog) = parse_store_response(wire) {
        assert!(catalog.official_supervisor_owner);
        assert!(!catalog.activation_authorized);
        assert!(catalog.apps.len() <= 50_000);
        assert!(catalog.repositories.len() <= 2048);

        let mut seen_apps = BTreeSet::new();
        for app in &catalog.apps {
            assert!(seen_apps.insert(app.slug.to_ascii_lowercase()));
            assert!(!app.slug.is_empty() && app.slug.len() <= 120);
            assert!(app.slug.bytes().all(|x|
                x.is_ascii_alphanumeric() || matches!(x, b'_' | b'-')
            ));
        }
        let mut seen_repositories = BTreeSet::new();
        for repo in &catalog.repositories {
            assert!(seen_repositories.insert(repo.slug.to_ascii_lowercase()));
            assert!(!repo.slug.is_empty() && repo.slug.len() <= 120);
        }
    }
});
