#![no_main]
// P8cz: adversarial synthetic App-only Supervisor TAR -> real native Rust
// read-only admission. No extraction, App activation, HAOS or real secrets.
use hars_interop_moli::app_backup_import::inspect_unprotected_app_only;
use libfuzzer_sys::fuzz_target;
use sha2::{Digest, Sha256};
use std::fs;

fuzz_target!(|input: &[u8]| {
    if !(1024..=2_097_152).contains(&input.len()) {
        return;
    }
    let private = tempfile::tempdir().expect("private fuzz workspace");
    let source = private.path().join("fictional-app-only.tar");
    fs::write(&source, input).expect("write isolated synthetic archive");
    if let Ok(proof) = inspect_unprotected_app_only(&source) {
        assert!((1..=8).contains(&proof.app_count));
        assert!(proof.files >= proof.app_count && proof.files <= 8 * 20_000);
        assert!(proof.declared_payload_bytes <= 8 * 268_435_456);
        assert_eq!(proof.archive_sha256, format!("{:x}", Sha256::digest(input)));
        assert!(proof.all_payloads_unprotected_gzip && proof.manifest_partial);
        assert!(!proof.core_migrated && !proof.app_runtime_migrated);
        assert!(!proof.actions_executable && !proof.production_ready);
        assert_eq!(fs::read(&source).expect("source remains readable"), input);
    }
});
