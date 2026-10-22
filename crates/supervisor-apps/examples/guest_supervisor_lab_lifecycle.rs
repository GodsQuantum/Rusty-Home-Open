//! Explicitly test-only, feature-gated authenticated official Supervisor
//! lifecycle operations. No production binary or UI routes include this.
use hars_supervisor_apps::official_transport::{AppSupervisorClient, LabAction};
fn main() {
    let mut args = std::env::args();
    let _program = args.next();
    let action = match (args.next().as_deref(), args.next()) {
        (Some("stop"), None) => LabAction::Stop,
        (Some("start"), None) => LabAction::Start,
        _ => {
            eprintln!("P8G: expected one lab fixture verb");
            std::process::exit(2);
        }
    };
    let result = AppSupervisorClient::from_environment()
        .and_then(|client| client.lab_fixture_transition(action));
    if let Err(kind) = result {
        // Only safe TransportError VARIANT; never print Supervisor body,
        // credentials, URL, device information or App configuration.
        eprintln!("P8G_LAB_FAILURE_KIND={kind:?}");
        std::process::exit(2);
    }
    let status = match action {
        LabAction::Stop => "STOP",
        LabAction::Start => "START",
    };
    println!("P8G_REAL_RUST_MANAGER_{status}_PASS");
}
