//! Supervisor-only ingress identity. Trust ONLY direct TCP from the
//! official Supervisor ingress proxy IP. Header values by themselves are
//! untrusted: clients can spoof X-Remote-User-Id on any other connection.
//! A verified identity is NOT permission to mutate Apps or authorize admin.
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub const SUPERVISOR_INGRESS_PROXY: Ipv4Addr = Ipv4Addr::new(172, 30, 32, 2);

#[derive(Debug, PartialEq, Eq)]
pub struct VerifiedIngress {
    pub route: String,
    pub user_id: String,
    /// Authority claimed by the official Supervisor's freshly resolved
    /// effective groups. Informational only: NEVER a permit for App writes.
    pub is_admin: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum IngressError {
    UntrustedPeer,
    InvalidHttp,
    MissingIdentity,
    DuplicateHeader,
    CrossSiteRequest,
}

pub fn verified_route(raw: &[u8], peer: SocketAddr) -> Result<VerifiedIngress, IngressError> {
    if peer.ip() != IpAddr::V4(SUPERVISOR_INGRESS_PROXY) {
        return Err(IngressError::UntrustedPeer);
    }
    if raw.len() > 4096 {
        return Err(IngressError::InvalidHttp);
    }
    let text = std::str::from_utf8(raw).map_err(|_| IngressError::InvalidHttp)?;
    let headers = text
        .strip_suffix("\r\n\r\n")
        .ok_or(IngressError::InvalidHttp)?;
    let mut lines = headers.split("\r\n");
    let line = lines.next().ok_or(IngressError::InvalidHttp)?;
    let mut parts = line.split(' ');
    let (Some("GET"), Some(route), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(IngressError::InvalidHttp);
    };
    if !matches!(version, "HTTP/1.1" | "HTTP/1.0")
        || !route.starts_with('/')
        || route.starts_with("//")
        || !route.bytes().all(|b| b.is_ascii_graphic())
    {
        return Err(IngressError::InvalidHttp);
    }
    let mut host = None;
    let mut user = None;
    let mut origin = None;
    let mut fetch = None;
    let mut groups = None;
    for (index, h) in lines.enumerate() {
        if index >= 64 || h.bytes().any(|b| !(32..=126).contains(&b)) {
            return Err(IngressError::InvalidHttp);
        }
        let (key, value) = h.split_once(':').ok_or(IngressError::InvalidHttp)?;
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return Err(IngressError::InvalidHttp);
        }
        let value = value.trim();
        if key.eq_ignore_ascii_case("host") {
            if host.replace(value).is_some() {
                return Err(IngressError::DuplicateHeader);
            }
            if value.is_empty()
                || value.len() > 256
                || !value.bytes().all(|b| {
                    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':' | b'[' | b']')
                })
            {
                return Err(IngressError::InvalidHttp);
            }
        } else if key.eq_ignore_ascii_case("x-remote-user-id") {
            if user.replace(value).is_some() {
                return Err(IngressError::DuplicateHeader);
            }
            if value.is_empty()
                || value.len() > 128
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            {
                return Err(IngressError::InvalidHttp);
            }
        } else if key.eq_ignore_ascii_case("x-remote-user-groups") {
            if groups.replace(value).is_some() {
                return Err(IngressError::DuplicateHeader);
            }
        } else if key.eq_ignore_ascii_case("origin") {
            if origin.replace(value).is_some() {
                return Err(IngressError::DuplicateHeader);
            }
        } else if key.eq_ignore_ascii_case("sec-fetch-site") {
            if fetch.replace(value).is_some() {
                return Err(IngressError::DuplicateHeader);
            }
        } else if key.eq_ignore_ascii_case("transfer-encoding")
            || (key.eq_ignore_ascii_case("content-length") && value != "0")
        {
            return Err(IngressError::InvalidHttp);
        }
    }
    let host = host.ok_or(IngressError::InvalidHttp)?;
    let user = user.ok_or(IngressError::MissingIdentity)?;
    if let Some(origin) = origin {
        if origin != format!("https://{host}") && origin != format!("http://{host}") {
            return Err(IngressError::CrossSiteRequest);
        }
    }
    if fetch.is_some_and(|f| !matches!(f, "same-origin" | "none")) {
        return Err(IngressError::CrossSiteRequest);
    }
    // Supervisor removes untrusted incoming user/group headers then sets
    // the canonical effective groups, including system-admin for an owner.
    // Never trust a group list from any other TCP peer; absent != admin.
    let mut is_admin = false;
    if let Some(raw) = groups {
        if raw.is_empty() || raw.len() > 256 {
            return Err(IngressError::InvalidHttp);
        }
        let mut seen = HashSet::new();
        for (idx, group) in raw.split(',').enumerate() {
            if idx >= 32
                || group.is_empty()
                || group.len() > 128
                || !group
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
                || !seen.insert(group)
            {
                return Err(IngressError::InvalidHttp);
            }
            if group == "system-admin" {
                is_admin = true;
            }
        }
    }
    Ok(VerifiedIngress {
        route: line.to_owned(),
        user_id: user.to_owned(),
        is_admin,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const TRUST: &str = "172.30.32.2:43002";
    fn get(raw: &str, peer: &str) -> Result<VerifiedIngress, IngressError> {
        verified_route(raw.as_bytes(), peer.parse().unwrap())
    }
    #[test]
    fn supervisor_proxy_identity_is_usable_for_all_read_routes() {
        for url in [
            "/",
            "/health",
            "/api/apps",
            "/api/sources",
            "/api/python-core",
        ] {
            let req=format!("GET {url} HTTP/1.1\r\nHost: home.example:8123\r\nX-Remote-User-Id: any_future_user_123\r\nSec-Fetch-Site: same-origin\r\nOrigin: https://home.example:8123\r\n\r\n");
            let id = get(&req, TRUST).unwrap();
            assert_eq!(id.route, format!("GET {url} HTTP/1.1"));
            assert_eq!(id.user_id, "any_future_user_123");
        }
    }
    #[test]
    fn forged_identity_on_any_non_supervisor_ip_never_authenticates() {
        let forged =
            "GET /api/apps HTTP/1.1\r\nHost: ha.example\r\nX-Remote-User-Id: admin\r\n\r\n";
        for peer in [
            "127.0.0.1:12345",
            "172.30.32.3:12345",
            "10.0.0.4:12345",
            "[::1]:12345",
        ] {
            assert_eq!(get(forged, peer), Err(IngressError::UntrustedPeer));
        }
        assert_eq!(get(forged, TRUST).unwrap().user_id, "admin");
    }
    #[test]
    fn only_exact_official_supervisor_admin_group_confers_readonly_admin_indicator() {
        let req = "GET /api/session HTTP/1.1\r\nHost: ha.example\r\nX-Remote-User-Id: owner_42\r\nX-Remote-User-Groups: system-users,system-admin\r\n\r\n";
        let verified = get(req, TRUST).unwrap();
        assert_eq!(verified.user_id, "owner_42");
        assert!(verified.is_admin);
        let non_admin = req.replace("system-users,system-admin", "system-users");
        assert!(!get(&non_admin, TRUST).unwrap().is_admin);
        let absent = req.replace("X-Remote-User-Groups: system-users,system-admin\r\n", "");
        assert!(!get(&absent, TRUST).unwrap().is_admin);
        assert_eq!(
            get(req, "172.30.32.3:43002"),
            Err(IngressError::UntrustedPeer)
        );
    }
    #[test]
    fn group_spoofing_and_malformed_group_lists_fail_closed() {
        let base = "GET /api/session HTTP/1.1\r\nHost: ha.example\r\nX-Remote-User-Id: test1\r\n";
        for value in [
            "system-admin,system-admin",
            "system-admin,",
            ",system-admin",
            "system-admin, system-users",
            "not/system-admin",
            "system-admin;admin",
            "system-admin,system-admin2,",
        ] {
            let input = format!("{base}X-Remote-User-Groups: {value}\r\n\r\n");
            assert!(get(&input, TRUST).is_err(), "{value}");
        }
        let repeated = format!("{base}X-Remote-User-Groups: system-admin\r\nx-remote-user-groups: system-admin\r\n\r\n");
        assert_eq!(get(&repeated, TRUST), Err(IngressError::DuplicateHeader));
        let almost = format!("{base}X-Remote-User-Groups: not-system-admin\r\n\r\n");
        assert!(!get(&almost, TRUST).unwrap().is_admin);
        let oversized = "g".repeat(260);
        let big = format!("{base}X-Remote-User-Groups: {oversized}\r\n\r\n");
        assert_eq!(get(&big, TRUST), Err(IngressError::InvalidHttp));
    }

    #[test]
    fn missing_identity_duplicate_headers_csrf_and_write_requests_fail_closed() {
        let missing = "GET /api/apps HTTP/1.1\r\nHost: ha.example\r\n\r\n";
        assert_eq!(get(missing, TRUST), Err(IngressError::MissingIdentity));
        let dual="GET /api/apps HTTP/1.1\r\nHost: ha.example\r\nX-Remote-User-Id: first\r\nx-remote-user-id: second\r\n\r\n";
        assert_eq!(get(dual, TRUST), Err(IngressError::DuplicateHeader));
        let req="GET /api/apps HTTP/1.1\r\nHost: ha.example\r\nX-Remote-User-Id: valid\r\nOrigin: https://evil.example\r\n\r\n";
        assert_eq!(get(req, TRUST), Err(IngressError::CrossSiteRequest));
        let write =
            "POST /api/apps HTTP/1.1\r\nHost: ha.example\r\nX-Remote-User-Id: admin\r\n\r\n";
        assert_eq!(get(write, TRUST), Err(IngressError::InvalidHttp));
        let host_dupe="GET /api/apps HTTP/1.1\r\nHost: ha.example\r\nHOST: ha.example\r\nX-Remote-User-Id: valid\r\n\r\n";
        assert_eq!(get(host_dupe, TRUST), Err(IngressError::DuplicateHeader));
        let smuggle="GET /api/apps HTTP/1.1\r\nHost: ha.example\r\nX-Remote-User-Id: valid\r\nTransfer-Encoding: chunked\r\n\r\n";
        assert_eq!(get(smuggle, TRUST), Err(IngressError::InvalidHttp));
    }
}
