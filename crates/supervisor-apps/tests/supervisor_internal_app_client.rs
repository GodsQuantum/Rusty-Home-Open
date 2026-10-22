//! P8e: official App-scoped Supervisor client, no arbitrary host supplied.
use hars_supervisor_apps::official_transport::{AppSupervisorClient, SupervisorToken};

#[test]
fn official_docker_endpoint_is_fixed_and_token_is_redacted() {
    let token = SupervisorToken::new("fixture-for-doc-tests-123".to_owned()).unwrap();
    let app = AppSupervisorClient::new(token);
    assert_eq!(app.service_name(), "supervisor");
    assert!(!format!("{app:?}").contains("fixture-for-doc-tests"));
    assert!(!format!("{app:?}").contains("123"));
}
