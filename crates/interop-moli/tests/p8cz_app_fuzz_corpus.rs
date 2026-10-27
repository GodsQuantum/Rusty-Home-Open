use hars_interop_moli::app_backup_import::inspect_unprotected_app_only;
use std::path::PathBuf;

fn corpus(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fuzz/corpus/app_backup_import")
        .join(name)
}

#[test]
fn accepts_fictional_single_and_two_app_backup_without_activation() {
    for (name, count) in [
        ("valid-one-fictional-app.tar", 1),
        ("valid-two-fictional-apps.tar", 2),
    ] {
        let report = inspect_unprotected_app_only(&corpus(name))
            .unwrap_or_else(|err| panic!("{name}: {err:#}"));
        assert_eq!(report.app_count, count);
        assert_eq!(report.files, count);
        assert!(!report.actions_executable && !report.production_ready);
    }
}

#[test]
fn denies_adversarial_app_only_archives() {
    let folder = corpus("valid-one-fictional-app.tar")
        .parent()
        .unwrap()
        .to_path_buf();
    let mut tested = 0;
    for entry in std::fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        if name.starts_with("reject-") {
            assert!(
                inspect_unprotected_app_only(&path).is_err(),
                "accepted {name}"
            );
            tested += 1;
        }
    }
    assert_eq!(tested, 12);
}
