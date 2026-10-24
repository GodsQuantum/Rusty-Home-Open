//! QEMU-only bridge between trusted live Core user status and existing durable
//! lab RBAC/one-shot audit. Not a production HTTP gateway or direct App write.
use crate::{
    audit_lab::{AuditedPermit, FileAuditJournal},
    core_user_live_lab::{CoreLiveError, CoreLiveResult, fetch_official_core_users_readonly},
    core_user_snapshot_lab::{CoreAccountStatus, CoreUserError},
    ingress_identity_lab::{
        IngressIdentityError, IngressUserClaim, ingress_user_claim_from_stream,
    },
    managed_apps::PreparedRequest,
    persistent_session_lab::DurableFixtureSessions,
    rbac_lab::{Policy, Role},
    session_lab::OpaqueToken,
};
use std::{io, net::TcpStream};

#[derive(Debug)]
pub enum CorroborationError {
    UntrustedIngress(IngressIdentityError),
    Core(CoreLiveError),
    InvalidCoreSnapshot(CoreUserError),
    DurableState(io::Error),
}
impl From<io::Error> for CorroborationError {
    fn from(e: io::Error) -> Self {
        Self::DurableState(e)
    }
}

/// All public callers MUST supply an accepted TCP connection, never caller
/// provided IP/Forwarded headers, and must recheck the current Core account.
/// The live Core adapter permits only the authenticated config/auth/list GET
/// equivalent, never a mutation or browser-selected host.
///
/// Important: successful fixture validation still cannot mint a production
/// capability. Preview transport remains compile- and QEMU-flag-gated.
#[allow(clippy::too_many_arguments)]
pub fn grant_corroborated_disposable_preview(
    accepted_tcp: &TcpStream,
    ingress_headers: &[(&str, &str)],
    policy: &Policy,
    sessions: &mut DurableFixtureSessions,
    session_token: &str,
    request: &PreparedRequest,
    confirmation: Option<&str>,
    journal: &mut FileAuditJournal,
    now: u64,
) -> Result<Option<AuditedPermit>, CorroborationError> {
    let Some(claim) = corroborated_active_admin(accepted_tcp, ingress_headers, now)? else {
        return Ok(None);
    };
    grant_checked_fixture_status(
        CoreAccountStatus::ActiveAdmin,
        &claim,
        policy,
        sessions,
        session_token,
        request,
        confirmation,
        journal,
        now,
    )
}

/// A fresh authenticated Core account check is required both when a
/// confirmation is issued and when that confirmation is redeemed.
fn corroborated_active_admin(
    accepted_tcp: &TcpStream,
    ingress_headers: &[(&str, &str)],
    now: u64,
) -> Result<Option<IngressUserClaim>, CorroborationError> {
    let claim = ingress_user_claim_from_stream(accepted_tcp, ingress_headers)
        .map_err(CorroborationError::UntrustedIngress)?;
    let status = match fetch_official_core_users_readonly(now).map_err(CorroborationError::Core)? {
        CoreLiveResult::PermissionDenied => return Ok(None),
        CoreLiveResult::Users(snapshot) => snapshot
            .status(&claim, now)
            .map_err(CorroborationError::InvalidCoreSnapshot)?,
    };
    Ok((status == CoreAccountStatus::ActiveAdmin).then_some(claim))
}

/// Laboratory only: authorization of a request-bound one-time confirmation
/// requires the *same* current live admin and active private session.
#[allow(clippy::too_many_arguments)]
pub fn issue_corroborated_disposable_confirmation(
    accepted_tcp: &TcpStream,
    ingress_headers: &[(&str, &str)],
    policy: &Policy,
    sessions: &mut DurableFixtureSessions,
    session_token: &str,
    request: &PreparedRequest,
    now: u64,
) -> Result<Option<OpaqueToken>, CorroborationError> {
    let Some(claim) = corroborated_active_admin(accepted_tcp, ingress_headers, now)? else {
        return Ok(None);
    };
    issue_checked_fixture_confirmation(
        CoreAccountStatus::ActiveAdmin,
        &claim,
        policy,
        sessions,
        session_token,
        request,
        now,
    )
}

fn issue_checked_fixture_confirmation(
    status: CoreAccountStatus,
    claim: &IngressUserClaim,
    policy: &Policy,
    sessions: &mut DurableFixtureSessions,
    token: &str,
    request: &PreparedRequest,
    now: u64,
) -> Result<Option<OpaqueToken>, CorroborationError> {
    if status != CoreAccountStatus::ActiveAdmin {
        return Ok(None);
    }
    let Some(actor) = sessions.current_fixture_actor(token, now)? else {
        return Ok(None);
    };
    if actor.subject() != claim.as_str() || actor.role() != Role::Administrator {
        return Ok(None);
    }
    sessions
        .issue_confirmation(policy, token, request, now)
        .map_err(Into::into)
}

/// Purely internal test seam: never accept Core status from an HTTP request.
/// Checks BOTH active Core admin and the subject of the unexpired persisted
/// session BEFORE minting a single-use audit-backed fixture permit.
#[allow(clippy::too_many_arguments)]
fn grant_checked_fixture_status(
    core_status: CoreAccountStatus,
    claim: &IngressUserClaim,
    policy: &Policy,
    sessions: &mut DurableFixtureSessions,
    session_token: &str,
    request: &PreparedRequest,
    confirmation: Option<&str>,
    journal: &mut FileAuditJournal,
    now: u64,
) -> Result<Option<AuditedPermit>, CorroborationError> {
    if core_status != CoreAccountStatus::ActiveAdmin {
        return Ok(None);
    }
    let Some(actor) = sessions.current_fixture_actor(session_token, now)? else {
        return Ok(None);
    };
    if actor.subject() != claim.as_str() || actor.role() != Role::Administrator {
        return Ok(None);
    }
    // Recheck session expiry and transactionally CONSUME confirmation even on
    // a wrong request; only grant_recorded_session can construct the permit.
    sessions
        .grant_recorded_session(policy, session_token, request, confirmation, journal, now)
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        audit_lab::FileAuditJournal,
        core_user_snapshot_lab::CoreUserSnapshot,
        ingress_identity_lab::ingress_user_claim,
        managed_apps::{Action, AppSlug, Operation, prepare},
        rbac_lab::{Actor, Policy},
    };
    use std::{
        fs,
        net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
        os::unix::fs::PermissionsExt,
    };
    fn claim(id: &str) -> IngressUserClaim {
        ingress_user_claim(
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(172, 30, 32, 2), 8045)),
            &[("X-Remote-User-Id", id)],
        )
        .unwrap()
    }
    fn core(status: &str) -> CoreUserSnapshot {
        let (active, group, system) = match status {
            "admin" => (true, "system-admin", false),
            "revoked" => (false, "system-admin", false),
            "demoted" => (true, "system-users", false),
            "machine" => (true, "system-admin", true),
            _ => panic!("unknown status"),
        };
        let wire = serde_json::json!({
            "type":"result","id":17,"success":true,"result":[{
                "id":"ownerabc","is_owner":false,"is_active":active,
                "system_generated":system,"group_ids":[group]
            }]
        });
        CoreUserSnapshot::parse_fixture(wire.to_string().as_bytes(), 17, 100).unwrap()
    }
    #[test]
    fn core_revocation_demotion_system_and_expiry_block_permit_before_consumption() {
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut sessions = DurableFixtureSessions::open(&dir.path().join("sessions.redb")).unwrap();
        let mut audit = FileAuditJournal::open(&dir.path().join("audit.jsonl")).unwrap();
        let app = AppSlug::parse("any_vendor_2026").unwrap();
        let req = prepare(&app, Operation::Stop).unwrap();
        let mut policy = Policy::default();
        policy.grant("ownerabc", &app, &[Action::Stop]);
        let session = sessions
            .issue_fixture(
                Actor::new("ownerabc", Role::Administrator).unwrap(),
                100,
                300,
                120,
            )
            .unwrap();
        let consent = sessions
            .issue_confirmation(&policy, session.as_str(), &req, 100)
            .unwrap()
            .unwrap();
        for status in ["revoked", "demoted", "machine"] {
            let fresh = core(status);
            assert!(
                grant_checked_fixture_status(
                    fresh.status(&claim("ownerabc"), 100).unwrap(),
                    &claim("ownerabc"),
                    &policy,
                    &mut sessions,
                    session.as_str(),
                    &req,
                    Some(consent.as_str()),
                    &mut audit,
                    100
                )
                .unwrap()
                .is_none()
            );
        }
        let verified = core("admin");
        assert!(
            grant_checked_fixture_status(
                verified.status(&claim("ownerabc"), 100).unwrap(),
                &claim("ownerabc"),
                &policy,
                &mut sessions,
                session.as_str(),
                &req,
                Some(consent.as_str()),
                &mut audit,
                100
            )
            .unwrap()
            .is_some()
        );
        // A legitimate one-time confirmation survives Core revocation but
        // cannot be replayed after use, including a reopened redb database.
        drop(sessions);
        let mut sessions = DurableFixtureSessions::open(&dir.path().join("sessions.redb")).unwrap();
        assert!(
            grant_checked_fixture_status(
                CoreAccountStatus::ActiveAdmin,
                &claim("ownerabc"),
                &policy,
                &mut sessions,
                session.as_str(),
                &req,
                Some(consent.as_str()),
                &mut audit,
                101
            )
            .unwrap()
            .is_none()
        );
        assert!(
            grant_checked_fixture_status(
                CoreAccountStatus::ActiveAdmin,
                &claim("ownerabc"),
                &policy,
                &mut sessions,
                session.as_str(),
                &req,
                None,
                &mut audit,
                221
            )
            .unwrap()
            .is_none()
        );
    }
    #[test]
    fn same_core_admin_cannot_use_another_subject_session_or_wrong_app_consent() {
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut sessions = DurableFixtureSessions::open(&dir.path().join("sessions.redb")).unwrap();
        let mut audit = FileAuditJournal::open(&dir.path().join("audit.jsonl")).unwrap();
        let app = AppSlug::parse("future_vendor_9").unwrap();
        let other = AppSlug::parse("other_vendor").unwrap();
        let req = prepare(&app, Operation::Install).unwrap();
        let other_req = prepare(&other, Operation::Install).unwrap();
        let mut policy = Policy::default();
        policy.grant("ownerabc", &app, &[Action::Install]);
        policy.grant("otheruser", &app, &[Action::Install]);
        let session = sessions
            .issue_fixture(
                Actor::new("otheruser", Role::Administrator).unwrap(),
                100,
                300,
                100,
            )
            .unwrap();
        assert!(
            grant_checked_fixture_status(
                CoreAccountStatus::ActiveAdmin,
                &claim("ownerabc"),
                &policy,
                &mut sessions,
                session.as_str(),
                &req,
                None,
                &mut audit,
                100
            )
            .unwrap()
            .is_none()
        );
        let mine = sessions
            .issue_fixture(
                Actor::new("ownerabc", Role::Administrator).unwrap(),
                100,
                300,
                100,
            )
            .unwrap();
        let consent = sessions
            .issue_confirmation(&policy, mine.as_str(), &req, 100)
            .unwrap()
            .unwrap();
        assert!(
            grant_checked_fixture_status(
                CoreAccountStatus::ActiveAdmin,
                &claim("ownerabc"),
                &policy,
                &mut sessions,
                mine.as_str(),
                &other_req,
                Some(consent.as_str()),
                &mut audit,
                100
            )
            .unwrap()
            .is_none()
        );
        // A mismatched approval is spent in the DB and cannot later be reused.
        assert!(
            grant_checked_fixture_status(
                CoreAccountStatus::ActiveAdmin,
                &claim("ownerabc"),
                &policy,
                &mut sessions,
                mine.as_str(),
                &req,
                Some(consent.as_str()),
                &mut audit,
                100
            )
            .unwrap()
            .is_none()
        );
        let new = sessions
            .issue_confirmation(&policy, mine.as_str(), &req, 100)
            .unwrap()
            .unwrap();
        assert!(sessions.revoke(mine.as_str()).unwrap());
        assert!(
            grant_checked_fixture_status(
                CoreAccountStatus::ActiveAdmin,
                &claim("ownerabc"),
                &policy,
                &mut sessions,
                mine.as_str(),
                &req,
                Some(new.as_str()),
                &mut audit,
                101
            )
            .unwrap()
            .is_none()
        );
        assert!(
            !format!(
                "{:?}",
                CorroborationError::Core(CoreLiveError::Unauthorized)
            )
            .contains(mine.as_str())
        );
    }
    #[test]
    fn core_status_checked_both_before_issuing_and_before_consuming_one_use_consent() {
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut sessions = DurableFixtureSessions::open(&dir.path().join("sessions.redb")).unwrap();
        let mut audit = FileAuditJournal::open(&dir.path().join("audit.jsonl")).unwrap();
        let app = AppSlug::parse("unknown_vendor_909").unwrap();
        let req = prepare(&app, Operation::Install).unwrap();
        let mut policy = Policy::default();
        policy.grant("ownerabc", &app, &[Action::Install]);
        let session = sessions
            .issue_fixture(
                Actor::new("ownerabc", Role::Administrator).unwrap(),
                100,
                180,
                90,
            )
            .unwrap();
        for role in [CoreAccountStatus::Denied, CoreAccountStatus::ActiveNonAdmin] {
            assert!(
                issue_checked_fixture_confirmation(
                    role,
                    &claim("ownerabc"),
                    &policy,
                    &mut sessions,
                    session.as_str(),
                    &req,
                    100
                )
                .unwrap()
                .is_none()
            );
        }
        assert!(
            issue_checked_fixture_confirmation(
                CoreAccountStatus::ActiveAdmin,
                &claim("otheradmin"),
                &policy,
                &mut sessions,
                session.as_str(),
                &req,
                100
            )
            .unwrap()
            .is_none()
        );
        let challenge = issue_checked_fixture_confirmation(
            CoreAccountStatus::ActiveAdmin,
            &claim("ownerabc"),
            &policy,
            &mut sessions,
            session.as_str(),
            &req,
            100,
        )
        .unwrap()
        .unwrap();
        // Revoked after issuance, before consumption: never grant.
        assert!(
            grant_checked_fixture_status(
                CoreAccountStatus::Denied,
                &claim("ownerabc"),
                &policy,
                &mut sessions,
                session.as_str(),
                &req,
                Some(challenge.as_str()),
                &mut audit,
                101
            )
            .unwrap()
            .is_none()
        );
        assert!(
            grant_checked_fixture_status(
                CoreAccountStatus::ActiveAdmin,
                &claim("ownerabc"),
                &policy,
                &mut sessions,
                session.as_str(),
                &req,
                Some(challenge.as_str()),
                &mut audit,
                101
            )
            .unwrap()
            .is_some()
        );
        assert!(
            grant_checked_fixture_status(
                CoreAccountStatus::ActiveAdmin,
                &claim("ownerabc"),
                &policy,
                &mut sessions,
                session.as_str(),
                &req,
                Some(challenge.as_str()),
                &mut audit,
                101
            )
            .unwrap()
            .is_none()
        );
    }
    #[test]
    fn confirmation_issuance_refuses_wrong_operation_and_expired_fixture() {
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut sessions = DurableFixtureSessions::open(&dir.path().join("sessions.redb")).unwrap();
        let app = AppSlug::parse("third_party_random").unwrap();
        let other = AppSlug::parse("ungranted_vendor").unwrap();
        let stop = prepare(&app, Operation::Stop).unwrap();
        let start = prepare(&app, Operation::Start).unwrap();
        let wrong = prepare(&other, Operation::Stop).unwrap();
        let mut policy = Policy::default();
        policy.grant("ownerabc", &app, &[Action::Start, Action::Stop]);
        let session = sessions
            .issue_fixture(
                Actor::new("ownerabc", Role::Administrator).unwrap(),
                100,
                120,
                80,
            )
            .unwrap();
        assert!(
            issue_checked_fixture_confirmation(
                CoreAccountStatus::ActiveAdmin,
                &claim("ownerabc"),
                &policy,
                &mut sessions,
                session.as_str(),
                &start,
                100
            )
            .unwrap()
            .is_none()
        );
        assert!(
            issue_checked_fixture_confirmation(
                CoreAccountStatus::ActiveAdmin,
                &claim("ownerabc"),
                &policy,
                &mut sessions,
                session.as_str(),
                &wrong,
                100
            )
            .unwrap()
            .is_none()
        );
        assert!(
            issue_checked_fixture_confirmation(
                CoreAccountStatus::ActiveAdmin,
                &claim("ownerabc"),
                &policy,
                &mut sessions,
                session.as_str(),
                &stop,
                100
            )
            .unwrap()
            .is_some()
        );
        assert!(
            issue_checked_fixture_confirmation(
                CoreAccountStatus::ActiveAdmin,
                &claim("ownerabc"),
                &policy,
                &mut sessions,
                session.as_str(),
                &stop,
                181
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn missing_real_supervisor_peer_never_trusts_forwarded_user_headers() {
        assert!(
            ingress_user_claim(
                SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8099),
                &[
                    ("X-Forwarded-For", "172.30.32.2"),
                    ("X-Remote-User-Id", "ownerabc")
                ]
            )
            .is_err()
        );
    }
}
