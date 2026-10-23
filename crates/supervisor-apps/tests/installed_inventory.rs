use hars_supervisor_apps::{
    installed_inventory::{InventoryError, RuntimeEvent, diff_running, parse_running_inventory},
    official_store::ApiVersion,
};
use serde_json::json;

fn data(v: ApiVersion, count: usize) -> Vec<u8> {
    let apps: Vec<_> = (0..count)
        .rev()
        .map(|i| {
            json!({
                "slug":format!("any_vendor_{i:04}"),
                "name":format!("Future Provider App {i}"),
                "repository":format!("future_repository_{}",i%19),
                "state":if i%2==0 {"started"} else {"stopped"},
                "version":format!("1.{}",i%9),
                "update_available":i%7==0,
                "options":{"password":"NEVER_OUTPUT"},
                "ip_address":"192.0.2.33"
            })
        })
        .collect();
    let key = match v {
        ApiVersion::V1 => "addons",
        ApiVersion::V2 => "apps",
    };
    serde_json::to_vec(&json!({"result":"ok","data":{(key):apps}})).unwrap()
}
#[test]
fn arbitrary_apps_and_providers_supported_in_both_versions() {
    for version in [ApiVersion::V1, ApiVersion::V2] {
        let report = parse_running_inventory(&data(version, 1501), version).unwrap();
        assert_eq!(report.apps.len(), 1501);
        assert_eq!(report.apps[0].slug, "any_vendor_0000");
        assert_eq!(report.apps[1500].slug, "any_vendor_1500");
        assert!(report.read_only);
        let text = serde_json::to_string(&report).unwrap();
        for private in [
            "NEVER_OUTPUT",
            "192.0.2.33",
            "password",
            "options",
            "ip_address",
        ] {
            assert!(!text.contains(private));
        }
    }
}
#[test]
fn updates_and_removals_are_real_state_differences() {
    let old = parse_running_inventory(&data(ApiVersion::V2, 30), ApiVersion::V2).unwrap();
    let mut new = old.clone();
    new.apps.remove(4);
    new.apps[0].state = Some("stopped".to_owned());
    new.apps[0].version = Some("99.0".to_owned());
    new.apps[0].update_available = Some(false);
    new.apps
        .push(hars_supervisor_apps::installed_inventory::RunningApp {
            slug: "new_vendor".into(),
            name: "Future App".into(),
            repository: None,
            state: Some("started".into()),
            version: None,
            update_available: None,
        });
    let changes = diff_running(&old, &new);
    assert!(
        changes
            .iter()
            .any(|c| c.event == RuntimeEvent::Installed && c.slug == "new_vendor")
    );
    assert!(changes.iter().any(|c| c.event == RuntimeEvent::Removed));
    assert!(
        changes
            .iter()
            .any(|c| c.event == RuntimeEvent::VersionChanged)
    );
    assert!(
        changes
            .iter()
            .any(|c| c.event == RuntimeEvent::StateChanged)
    );
    assert!(
        changes
            .iter()
            .any(|c| c.event == RuntimeEvent::UpdateChanged)
    );
}
#[test]
fn rejects_duplicate_slug_wrong_version_and_untrusted_responses() {
    let v1 = data(ApiVersion::V1, 5);
    assert_eq!(
        parse_running_inventory(&v1, ApiVersion::V2),
        Err(InventoryError::WrongApiVersion)
    );
    let mut duplicated = serde_json::from_slice::<serde_json::Value>(&v1).unwrap();
    let first = duplicated["data"]["addons"][0].clone();
    duplicated["data"]["addons"]
        .as_array_mut()
        .unwrap()
        .push(first);
    assert_eq!(
        parse_running_inventory(&serde_json::to_vec(&duplicated).unwrap(), ApiVersion::V1),
        Err(InventoryError::Duplicate)
    );
    for bad in [
        br#"{"result":"error","data":{"apps":[]}}"#.as_slice(),
        br#"{"result":"ok","data":{"apps":[],"addons":[]}}"#,
        br#"{"result":"ok","data":{"apps":[{"slug":"safe","slug":"evil","name":"N"}]}}"#,
        br#"{"result":"ok","data":{"apps":[{"slug":"../../evil","name":"N"}]}}"#,
        br#"{"result":"ok","data":{"apps":[{"slug":"safe","name":"N","update_available":"true"}]}}"#,
    ] {
        assert!(parse_running_inventory(bad,ApiVersion::V2).is_err());
    }
}
