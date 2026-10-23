//! Real official Supervisor REST call, intended only for a disposable HAOS
//! integration-test guest and run inside a Supervisor-provisioned App.
use hars_supervisor_apps::{
    official_store::ApiVersion,
    official_transport::{AppSupervisorClient, app_service_mode},
};
use std::io::Write;
fn main() {
    let outcome = AppSupervisorClient::from_environment().and_then(|client| {
        let catalog = client.fetch_store(ApiVersion::V1)?;
        // Genuine, authenticated HAOS QEMU GET /jobs/info, not a mock.
        let jobs = client.fetch_jobs(ApiVersion::V1)?;
        Ok((catalog, jobs))
    });
    match outcome {
        Ok((catalog, jobs)) if !catalog.repositories.is_empty() && !catalog.apps.is_empty() => {
            // Prints only aggregates, never URLs, tokens, store names or secrets.
            println!(
                "P8E_REAL_RUST_SUPERVISOR_AUTH_PASS repos={} apps={} readonly={} jobs={} jobs_readonly={}",
                catalog.repositories.len(),
                catalog.apps.len(),
                !catalog.activation_authorized,
                jobs.jobs.len(),
                jobs.read_only
            );
            // The CLI's one-shot probe still returns immediately. Only the
            // dedicated Supervisor App stays alive after a successful GET.
            if app_service_mode(std::env::var("HARS_PROBE_STAY_RUNNING").ok().as_deref()) {
                let _ = std::io::stdout().flush();
                loop {
                    std::thread::park();
                }
            }
        }
        _ => {
            eprintln!("P8E: denied/invalid Supervisor response (no credentials logged)");
            std::process::exit(2);
        }
    }
}
