use hars_community_packages::marketplace_ws::{
    Category, ReadCommand, ReadError, core_marketplace_supported, parse_read_result, read_request,
};
use serde_json::json;
#[test]
fn version_boundary() {
    for v in [
        "2026.10.0",
        "2026.10.9",
        "2026.09.0",
        "",
        "2026.1.0",
        "2026.13.0",
        "2026.11",
        "2026.11.0/../",
        "bad",
    ] {
        assert!(!core_marketplace_supported(v), "{v}");
    }
    for v in [
        "2026.11.0",
        "2026.11.0b0",
        "2026.11.0dev0",
        "2026.12.1",
        "2027.01.0",
    ] {
        assert!(core_marketplace_supported(v), "{v}");
    }
}
#[test]
fn official_core_read_commands_exact_keys() {
    assert_eq!(
        read_request("2026.11.0", 1, ReadCommand::Info).unwrap(),
        json!({"id":1,"type":"marketplace/info"})
    );
    assert_eq!(
        read_request("2026.11.0b0", 9, ReadCommand::List { categories: vec![] }).unwrap(),
        json!({"id":9,"type":"marketplace/repositories/list"})
    );
    assert_eq!(
        read_request(
            "2026.11.0",
            3,
            ReadCommand::List {
                categories: vec![
                    Category::Integration,
                    Category::Plugin,
                    Category::Theme,
                    Category::Template
                ]
            }
        )
        .unwrap(),
        json!({"id":3,"type":"marketplace/repositories/list","categories":["integration","plugin","theme","template"]})
    );
    assert_eq!(
        read_request(
            "2026.11.0",
            17,
            ReadCommand::RepositoryInfo {
                repository_id: "12345".into()
            }
        )
        .unwrap(),
        json!({"id":17,"type":"marketplace/repository/info","repository_id":"12345"})
    );
}
#[test]
fn rejects_injection_duplication_and_legacy() {
    assert_eq!(
        read_request("2026.10.4", 1, ReadCommand::Info),
        Err(ReadError::UnsupportedVersion)
    );
    assert_eq!(
        read_request("2026.11.0", 0, ReadCommand::Info),
        Err(ReadError::InvalidRequestId)
    );
    assert_eq!(
        read_request(
            "2026.11.0",
            4,
            ReadCommand::List {
                categories: vec![Category::Theme, Category::Theme]
            }
        ),
        Err(ReadError::DuplicateCategory)
    );
    for id in [
        "",
        "../secret",
        "http://host",
        "owner/repo",
        "abc?x=1",
        " space",
        "a\nb",
        ".hidden",
    ] {
        assert_eq!(
            read_request(
                "2026.11.0",
                4,
                ReadCommand::RepositoryInfo {
                    repository_id: id.into()
                }
            ),
            Err(ReadError::InvalidRepositoryId)
        );
    }
    let oversized = "a".repeat(129);
    assert_eq!(
        read_request(
            "2026.11.0",
            4,
            ReadCommand::RepositoryInfo {
                repository_id: oversized
            }
        ),
        Err(ReadError::InvalidRepositoryId)
    );
}
#[test]
fn fail_closed_on_wrong_id_unauthorized_or_events() {
    let good = json!({"id":81,"type":"result","success":true,"result":{"github_connected":false}});
    assert_eq!(
        parse_read_result(81, &good).unwrap()["github_connected"],
        false
    );
    for reply in [
        json!({"id":82,"type":"result","success":true,"result":[]}),
        json!({"id":81,"type":"event","event":{"x":"bad"}}),
        json!({"id":81,"type":"result","success":false,"error":{"code":"unauthorized"}}),
        json!({"id":81,"type":"result","success":true}),
        json!({"id":81,"type":"result","success":1,"result":[]}),
    ] {
        assert!(parse_read_result(81, &reply).is_err());
    }
}
