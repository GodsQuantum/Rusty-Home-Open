#![no_main]

// Run the ACTUAL product ingress parser, not a reimplementation of the
// protocol or a mocked policy. No networking, tokens, users or file I/O.
#[path = "../../crates/control-plane/src/ingress.rs"]
mod ingress;

use libfuzzer_sys::fuzz_target;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fuzz_target!(|wire: &[u8]| {
    let trusted = SocketAddr::new(
        IpAddr::V4(ingress::SUPERVISOR_INGRESS_PROXY), 49152
    );
    let untrusted = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 20)), 49152
    );
    // A spoofed header never upgrades an untrusted TCP source.
    assert_eq!(
        ingress::verified_route(wire, untrusted),
        Err(ingress::IngressError::UntrustedPeer)
    );
    if let Ok(id) = ingress::verified_route(wire, trusted) {
        assert!(id.route.starts_with("GET /"));
        assert!(id.route.ends_with(" HTTP/1.1") || id.route.ends_with(" HTTP/1.0"));
        assert!(!id.user_id.is_empty() && id.user_id.len() <= 128);
        assert!(id.user_id.bytes().all(|c|
            c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-')
        ));
    }
});
