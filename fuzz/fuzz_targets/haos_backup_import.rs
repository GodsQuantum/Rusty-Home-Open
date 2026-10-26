#![no_main]
// Real unprotected Supervisor full tar -> Rust registry + raw YAML parser.
// Every corpus byte is synthetic. Never touches HAOS, network or devices.
use hars_interop_moli::{
    ha_backup_import::registry_from_unprotected_backup, registry_stage::parse_export,
};
use libfuzzer_sys::fuzz_target;
use std::fs;

fuzz_target!(|input: &[u8]| {
    if !(1024..=1_048_576).contains(&input.len()) {
        return;
    }
    // File in a fresh owner-private TEMP directory; follow no links. The
    // production importer verifies inode, archive framing, gzip and members.
    let dir = tempfile::tempdir().expect("private temporary directory");
    let archive = dir.path().join("synthetic-supervisor.tar");
    fs::write(&archive, input).expect("write synthetic archive");
    if let Ok(result) = registry_from_unprotected_backup(&archive) {
        let parsed =
            parse_export(&result.raw).expect("accepted archive must contain validated registry");
        assert!(parsed.read_only && parsed.no_device_commands && parsed.no_live_states);
        assert_eq!(result.archive_sha256.len(), 64);
        if let Some(raw) = result.automations_yaml {
            assert!(!raw.is_empty() && raw.len() <= 4 * 1024 * 1024);
            assert!(std::str::from_utf8(&raw).is_ok());
        }
    }
});
