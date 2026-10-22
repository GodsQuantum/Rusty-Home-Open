//! Laboratory-only admission of *claims* carried by Supervisor Ingress.
//! NOT a login provider, a role grant, or a direct HTTP listener.
//! Production must additionally verify current Core user rights, require
//! Supervisor-only network routing, and implement an authenticated UI.
//! Official Ingress proxy strips client-controlled X-Remote-User-* and writes
//! X-Remote-User-Id only when a Core user is attached to the session.
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};

const SUPERVISOR_INGRESS_IP: Ipv4Addr = Ipv4Addr::new(172, 30, 32, 2);
const MAX_HEADERS: usize = 64;
const MAX_HEADER_VALUE: usize = 4096;
const MAX_ID_LEN: usize = 128;

#[derive(PartialEq, Eq, Clone)]
pub struct IngressUserClaim(String);
impl IngressUserClaim {
    /// An *identity claim*, not an authenticated user/role/authorization.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for IngressUserClaim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("IngressUserClaim([REDACTED])")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngressIdentityError {
    UntrustedPeer,
    MalformedHeaders,
    MissingIdentity,
    AmbiguousIdentity,
    InvalidIdentity,
}

/// Entry point for a future real ingress listener: derive peer identity
/// directly from an accepted TCP stream, never from request metadata.
/// This does not perform authentication, user permission lookup or a write.
pub fn ingress_user_claim_from_stream(
    accepted_tcp: &TcpStream,
    headers: &[(&str, &str)],
) -> Result<IngressUserClaim, IngressIdentityError> {
    let peer = accepted_tcp
        .peer_addr()
        .map_err(|_| IngressIdentityError::UntrustedPeer)?;
    ingress_user_claim(peer, headers)
}

/// Inspect the actual remote IP of the ACCEPTED socket (not Forwarded headers).
///
/// Caller MUST capture SocketAddr from the TCP listener, never from URL, JSON,
/// X-Forwarded-For or X-Real-IP. Only exact Supervisor Ingress peer is trusted.
/// This function does NOT validate the user's active status or permission.
pub(crate) fn ingress_user_claim(
    tcp_peer: SocketAddr,
    headers: &[(&str, &str)],
) -> Result<IngressUserClaim, IngressIdentityError> {
    if tcp_peer.ip() != IpAddr::V4(SUPERVISOR_INGRESS_IP) {
        return Err(IngressIdentityError::UntrustedPeer);
    }
    if headers.len() > MAX_HEADERS {
        return Err(IngressIdentityError::MalformedHeaders);
    }
    let mut id = None;
    for &(name, value) in headers {
        if name.is_empty()
            || name.len() > 128
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || value.len() > MAX_HEADER_VALUE
            || value.bytes().any(|b| b.is_ascii_control())
        {
            return Err(IngressIdentityError::MalformedHeaders);
        }
        if name.eq_ignore_ascii_case("X-Remote-User-Id") {
            if id.is_some() {
                return Err(IngressIdentityError::AmbiguousIdentity);
            }
            if value.is_empty()
                || value.len() > MAX_ID_LEN
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            {
                return Err(IngressIdentityError::InvalidIdentity);
            }
            id = Some(value.to_owned());
        }
    }
    id.map(IngressUserClaim)
        .ok_or(IngressIdentityError::MissingIdentity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
    fn identity(headers: &[(&str, &str)]) -> Result<IngressUserClaim, IngressIdentityError> {
        ingress_user_claim(
            SocketAddr::V4(SocketAddrV4::new(SUPERVISOR_INGRESS_IP, 8099)),
            headers,
        )
    }
    #[test]
    fn only_exact_supervisor_tcp_peer_and_single_user_id_are_accepted() {
        let user = identity(&[
            ("Host", "localhost:8099"),
            ("X-Remote-User-Id", "4a8b1f64d8bc4ed2a8ed76afefc206aa"),
            ("X-Remote-User-Name", "Fake Admin"),
        ])
        .unwrap();
        assert_eq!(user.as_str(), "4a8b1f64d8bc4ed2a8ed76afefc206aa");
        assert!(!format!("{user:?}").contains("Fake Admin"));
        // Caller can only obtain an identity claim, never a role/permit.
        for addr in [
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8099)),
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(172, 30, 32, 3), 80)),
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 217), 443)),
            SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 8099, 0, 0)),
        ] {
            assert!(matches!(
                ingress_user_claim(addr, &[("X-Remote-User-Id", "valid-id")]),
                Err(IngressIdentityError::UntrustedPeer)
            ));
        }
    }
    #[test]
    fn direct_and_forwarded_headers_never_override_tcp_peer() {
        assert!(matches!(
            ingress_user_claim(
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 80)),
                &[
                    ("X-Forwarded-For", "172.30.32.2"),
                    ("X-Remote-User-Id", "user1"),
                ]
            ),
            Err(IngressIdentityError::UntrustedPeer)
        ));
        assert_eq!(
            identity(&[
                ("X-Forwarded-For", "198.51.100.42"),
                ("X-Remote-User-Id", "legitimate2"),
            ])
            .unwrap()
            .as_str(),
            "legitimate2"
        );
    }
    #[test]
    fn missing_duplicate_or_invalid_user_header_is_denied_not_anonymous_admin() {
        for headers in [
            vec![],
            vec![("X-Remote-User-Name", "admin")],
            vec![("X-Remote-User-Id", "")],
            vec![("X-Remote-User-Id", " admin")],
            vec![("X-Remote-User-Id", "admin,user")],
            vec![("X-Remote-User-Id", "admin\r\nX-Role:admin")],
            vec![("X-Remote-User-Id", "../../admin")],
            vec![("X-Remote-User-Id", "\u{1F600}")],
            vec![("X-Remote-User-Id", "admin"), ("x-REMOTE-user-ID", "other")],
            vec![("X-Remote-User-Id", "admin"), ("X-Remote-User-Id", "admin")],
            vec![("X-Remote-User-Id", "a".repeat(129).leak())],
        ] {
            assert!(
                identity(&headers).is_err(),
                "unsafe ingress accepted: {headers:?}"
            );
        }
    }
    #[test]
    fn excessive_or_malformed_headers_fail_closed() {
        let headers = vec![("X-Remote-User-Id", "user1"); 65];
        assert!(matches!(
            identity(&headers),
            Err(IngressIdentityError::MalformedHeaders)
        ));
        assert!(identity(&[("Bad Header", "a"), ("X-Remote-User-Id", "user1")]).is_err());
        assert!(identity(&[("X-Remote-User-Id", "user1"), ("Content-Length", "3\r\n")]).is_err());
    }
}
