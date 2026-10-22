#![cfg(feature = "ingress-identity-lab")]
use hars_supervisor_apps::ingress_identity_lab::{
    IngressIdentityError, ingress_user_claim_from_stream,
};
#[test]
fn accepted_tcp_socket_peer_is_checked_not_injected_forwarded_address() {
    use std::net::{TcpListener, TcpStream};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (accepted, _) = listener.accept().unwrap();
    let forged = [
        ("X-Forwarded-For", "172.30.32.2"),
        ("X-Remote-User-Id", "fake-admin"),
    ];
    assert!(matches!(
        ingress_user_claim_from_stream(&accepted, &forged),
        Err(IngressIdentityError::UntrustedPeer)
    ));
    assert!(matches!(
        ingress_user_claim_from_stream(&client, &forged),
        Err(IngressIdentityError::UntrustedPeer)
    ));
}
