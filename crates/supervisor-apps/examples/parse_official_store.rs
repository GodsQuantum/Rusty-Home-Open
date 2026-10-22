//! Offline native Rust bridge contract against official Python Supervisor API
//! v1/v2. Reads only stdin; no HTTP, token, Docker socket or installation.
use std::io::{self, Read};
fn main() {
    if nix::unistd::Uid::effective().is_root() {
        eprintln!("REFUSED: upstream contract lab may not run as root");
        std::process::exit(2);
    }
    let mut bytes = Vec::new();
    if io::stdin()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .is_err()
    {
        std::process::exit(2);
    }
    match hars_supervisor_apps::official_store::parse_store_response(&bytes) {
        Ok(store) => println!("{}", serde_json::to_string(&store).expect("typed payload")),
        Err(_err) => {
            eprintln!("REFUSED: invalid Supervisor API contract; no authority granted");
            std::process::exit(2);
        }
    }
}
