//! Tests for opt-in, compiled-out-by-default Supervisor lifecycle lab.
#[cfg(feature = "lab-app-actions")]
use hars_supervisor_apps::official_transport::{
    AppSupervisorClient, LabAction, SupervisorToken, lab_action_io_timeout, lab_action_path,
    lab_mutation_enabled,
};
#[cfg(feature = "lab-app-actions")]
#[test]
fn fixed_upstream_routes_are_only_core_ssh_stop_and_start() {
    assert_eq!(lab_action_path(LabAction::Stop), "/addons/core_ssh/stop");
    assert_eq!(lab_action_path(LabAction::Start), "/addons/core_ssh/start");
}
#[cfg(feature = "lab-app-actions")]
#[test]
fn external_target_and_privilege_string_never_accepted() {
    assert!(!lab_mutation_enabled(None));
    assert!(!lab_mutation_enabled(Some("yes")));
    assert!(!lab_mutation_enabled(Some("true")));
    assert!(lab_mutation_enabled(Some("1")));
}
#[cfg(feature = "lab-app-actions")]
#[test]
fn real_client_refuses_mutation_without_disposable_lab_marker() {
    if std::env::var("HARS_LAB_MUTATIONS").is_err() {
        let client = AppSupervisorClient::new(
            SupervisorToken::new("fixture-token-no-real-data".to_owned()).unwrap(),
        );
        assert!(client.lab_fixture_transition(LabAction::Stop).is_err());
    }
}

#[cfg(feature = "lab-app-actions")]
#[test]
fn long_real_docker_lifecycle_operations_get_bounded_non_get_timeout() {
    let seconds = lab_action_io_timeout().as_secs();
    assert!(
        (60..=120).contains(&seconds),
        "POST must not use 3s GET timeout"
    );
}
