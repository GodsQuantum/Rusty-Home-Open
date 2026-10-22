//! Test-only binary. Never prints private states, secrets, or token values.
use hars_python_bridge::fetch_snapshot;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
fn main() {
    let mut args = std::env::args_os().skip(1);
    let Some(socket) = args.next() else {
        eprintln!("usage: hars-python-bridge SOCKET_PRIVATE_PATH CAPABILITY_FILE");
        std::process::exit(2);
    };
    let Some(capability) = args.next() else {
        eprintln!("missing capability file");
        std::process::exit(2);
    };
    if args.next().is_some() {
        std::process::exit(2);
    }
    let token_path = PathBuf::from(capability);
    let uid = match std::fs::symlink_metadata(&token_path) {
        Ok(m) => m.uid(),
        Err(_) => {
            eprintln!("capability inaccessible");
            std::process::exit(2)
        }
    };
    match fetch_snapshot(&PathBuf::from(socket), &token_path, uid) {
        Ok(snapshot) => println!(
            "PASS: Python bridge read-only snapshot generation={} states={}",
            snapshot.generation,
            snapshot.entities.len()
        ),
        Err(_) => {
            eprintln!("FAIL: Python bridge denied/invalid response");
            std::process::exit(1)
        }
    }
}
