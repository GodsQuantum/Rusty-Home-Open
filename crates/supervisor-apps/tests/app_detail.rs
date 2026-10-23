use hars_supervisor_apps::{
    app_detail::{InfoError, detail_path, parse_app_detail},
    official_store::ApiVersion,
};
use serde_json::json;

#[test]
fn arbitrary_future_app_detail_uses_official_versioned_api_paths() {
    assert_eq!(
        detail_path(ApiVersion::V1, "brand_new_provider").unwrap(),
        "/addons/brand_new_provider/info"
    );
    assert_eq!(
        detail_path(ApiVersion::V2, "brand_new_provider").unwrap(),
        "/v2/apps/brand_new_provider/info"
    );
    for slug in ["", "../secret", "my/app", "app?x=1", "a\r\nHost:x"] {
        assert_eq!(
            detail_path(ApiVersion::V1, slug),
            Err(InfoError::UnsafeSlug)
        );
    }
}
#[test]
fn sanitizes_secrets_and_preserves_generic_actual_data() {
    let value = json!({"result":"ok","data":{
        "slug":"future_vendor_app","name":"Unknown Future App","repository":"future_repo",
        "state":"started","version":"18.7.0","version_latest":"18.9.0",
        "available":true,"update_available":true,"ingress":true,"system_managed":false,
        "options":{"private_password":"NEVER_LEAK"},"schema":{"token":"SECRET"},
        "ip_address":"192.0.2.190","devices":["/dev/secret"],
        "network":{"8123/tcp":"SECRET"}
    }});
    let app = parse_app_detail(&serde_json::to_vec(&value).unwrap(), "future_vendor_app").unwrap();
    assert_eq!(app.slug, "future_vendor_app");
    assert_eq!(app.repository.as_deref(), Some("future_repo"));
    assert_eq!(app.version.as_deref(), Some("18.7.0"));
    assert!(app.update_available.unwrap());
    assert!(app.ingress.unwrap());
    assert!(app.official_supervisor_owner && !app.activation_authorized);
    let serialized = serde_json::to_string(&app).unwrap();
    for sensitive in [
        "NEVER_LEAK",
        "SECRET",
        "ip_address",
        "devices",
        "options",
        "schema",
        "network",
    ] {
        assert!(!serialized.contains(sensitive));
    }
}
#[test]
fn fails_closed_on_forged_identifiers_bad_status_and_duplicate_json() {
    let data = r#"{"result":"ok","data":{"slug":"future","name":"Safe","options":{"secret":"x"}}}"#;
    assert!(parse_app_detail(data.as_bytes(), "future").is_ok());
    assert_eq!(
        parse_app_detail(data.as_bytes(), "other"),
        Err(InfoError::MismatchedSlug)
    );
    for invalid in [
        r#"{"result":"error","data":{"slug":"future","name":"Safe"}}"#,
        r#"{"result":"ok","data":{"slug":"future","name":"Safe","name":"Evil"}}"#,
        r#"{"result":"ok","data":{"slug":"future","name":"Safe","available":"yes"}}"#,
        r#"{"result":"ok","data":{"slug":"future","name":"Bad\nName"}}"#,
        r#"{"result":"ok","data":{"slug":"future","name":""}}"#,
    ] {
        assert!(
            parse_app_detail(invalid.as_bytes(), "future").is_err(),
            "{invalid}"
        );
    }
    assert_eq!(
        parse_app_detail(&vec![b'x'; 512 * 1024 + 1], "future"),
        Err(InfoError::TooLarge)
    );
}

#[test]
fn store_detail_paths_support_uninstalled_future_apps_but_not_traversal() {
    use hars_supervisor_apps::app_detail::store_detail_path;
    assert_eq!(
        store_detail_path(ApiVersion::V1, "unseen_vendor_app").unwrap(),
        "/store/addons/unseen_vendor_app"
    );
    assert_eq!(
        store_detail_path(ApiVersion::V2, "unseen_vendor_app").unwrap(),
        "/v2/store/apps/unseen_vendor_app"
    );
    for slug in ["", "../secret", "bad/path", "bad?query=1", "bad%2fslug"] {
        assert_eq!(
            store_detail_path(ApiVersion::V1, slug),
            Err(InfoError::UnsafeSlug)
        );
    }
}
