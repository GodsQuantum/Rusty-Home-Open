use hars_supervisor_apps::official_transport::app_service_mode;
#[test]
fn opt_in_only_explicit_service_mode_not_cli_probe() {
    assert!(!app_service_mode(None));
    assert!(!app_service_mode(Some("")));
    assert!(!app_service_mode(Some("0")));
    assert!(!app_service_mode(Some("true")));
    assert!(app_service_mode(Some("1")));
}
