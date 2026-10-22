//! QEMU-ONLY read-only probe of genuine Supervisor/Core WebSocket.
//! Emits categorical markers only. Never prints token, user IDs or raw frames.
use hars_supervisor_apps::core_user_live_lab::{
    CoreLiveError, CoreLiveResult, fetch_official_core_users_qemu,
};
fn main() {
    let probe = std::env::args().nth(1).as_deref() == Some("--probe");
    if !probe
        && std::env::var("HARS_CORE_PROBE_STAY_RUNNING")
            .ok()
            .as_deref()
            == Some("1")
    {
        loop {
            std::thread::park();
        }
    }
    if !probe {
        eprintln!("P8I_CORE_PROBE_EXPLICIT_ACTION_REQUIRED");
        std::process::exit(2);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("lab clock before Unix epoch")
        .as_secs();
    match fetch_official_core_users_qemu(now) {
        Ok(CoreLiveResult::Users(_)) => println!("P8I_REAL_CORE_WS_USERS_READ"),
        Ok(CoreLiveResult::PermissionDenied) => println!("P8I_REAL_CORE_WS_LIST_DENIED"),
        Err(CoreLiveError::Unauthorized) => {
            eprintln!("P8I_REAL_CORE_WS_AUTH_REJECTED");
            std::process::exit(3);
        }
        Err(_) => {
            eprintln!("P8I_REAL_CORE_WS_TRANSPORT_NOT_VERIFIED");
            std::process::exit(4);
        }
    }
}
