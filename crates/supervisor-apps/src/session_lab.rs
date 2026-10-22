//! P8i.d.1: laboratory-only server-side session / single-use confirmation model.
//! Caller establishes actors with FIXTURES; this is not a real identity provider.
//! Default application builds do not compile this module. The registry is
//! in-memory and is NOT persistent/revocation-safe across a process restart.
use crate::audit_lab::{AuditedPermit, FileAuditJournal, grant_recorded};
use crate::managed_apps::PreparedRequest;
use crate::rbac_lab::{Acknowledgement, Actor, DecisionCode, Policy};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::io;

const MAX_TTL: u64 = 3_600;
const MAX_IDLE: u64 = 900;
const CONFIRM_TTL: u64 = 60;
const MAX_SESSIONS: usize = 1_024;
const MAX_APPROVALS: usize = 4_096;

#[derive(Clone)]
pub struct OpaqueToken(String);

impl OpaqueToken {
    /// Tokens are bearer secrets. Do not log them or include in URL parameters.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for OpaqueToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OpaqueToken([REDACTED])")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SessionError {
    InvalidLifetime,
    Capacity,
    Randomness,
    NoSession,
    NotPermitted,
}

struct Session {
    actor: Actor,
    expires_at: u64,
    idle: u64,
    last_seen: u64,
}

struct Approval {
    session_hash: [u8; 32],
    request: PreparedRequest,
    expires_at: u64,
}

#[derive(Default)]
pub struct LabSessionRegistry {
    sessions: BTreeMap<[u8; 32], Session>,
    approvals: BTreeMap<[u8; 32], Approval>,
}

impl fmt::Debug for LabSessionRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LabSessionRegistry")
            .field("session_count", &self.sessions.len())
            .field("approval_count", &self.approvals.len())
            .finish()
    }
}

pub(crate) fn digest_secret(raw: &str) -> Option<[u8; 32]> {
    if raw.len() != 64
        || !raw
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return None;
    }
    Some(Sha256::digest(raw.as_bytes()).into())
}

pub(crate) fn fresh_token() -> Result<OpaqueToken, SessionError> {
    let mut entropy = [0u8; 32];
    getrandom::fill(&mut entropy).map_err(|_| SessionError::Randomness)?;
    use std::fmt::Write as _;
    let mut token = String::with_capacity(64);
    for octet in entropy {
        write!(token, "{octet:02x}").expect("formatting into String");
    }
    Ok(OpaqueToken(token))
}

impl LabSessionRegistry {
    /// ONLY for isolated fixture actors; external login / OAuth is not wired.
    /// now is fixture-supplied, not an authoritative wall-clock or monotonic source.
    pub fn issue_fixture(
        &mut self,
        actor: Actor,
        now: u64,
        ttl: u64,
        idle: u64,
    ) -> Result<OpaqueToken, SessionError> {
        if ttl == 0 || ttl > MAX_TTL || idle == 0 || idle > MAX_IDLE || idle > ttl {
            return Err(SessionError::InvalidLifetime);
        }
        let expires_at = now.checked_add(ttl).ok_or(SessionError::InvalidLifetime)?;
        if self.sessions.len() >= MAX_SESSIONS {
            return Err(SessionError::Capacity);
        }
        let token = fresh_token()?;
        let key = digest_secret(token.as_str()).ok_or(SessionError::Randomness)?;
        if self.sessions.contains_key(&key) {
            return Err(SessionError::Randomness);
        }
        self.sessions.insert(
            key,
            Session {
                actor,
                expires_at,
                idle,
                last_seen: now,
            },
        );
        Ok(token)
    }

    /// Expiry, idle timeout and session revocation rechecked on every attempt.
    /// Rewinding the supplied clock is also denied (laboratory simulation).
    fn resolve(&mut self, raw: &str, now: u64) -> Option<([u8; 32], Actor)> {
        let hash = digest_secret(raw)?;
        let session = self.sessions.get_mut(&hash)?;
        if now < session.last_seen
            || now >= session.expires_at
            || now - session.last_seen >= session.idle
        {
            self.sessions.remove(&hash);
            return None;
        }
        session.last_seen = now;
        Some((hash, session.actor.clone()))
    }

    pub fn revoke(&mut self, raw: &str) -> bool {
        digest_secret(raw)
            .and_then(|hash| self.sessions.remove(&hash))
            .is_some()
    }

    /// Generates a short-lived confirmation, bound to session + exact target
    /// operation, only after *current* policy grants permit an acknowledgement.
    pub fn issue_confirmation(
        &mut self,
        policy: &Policy,
        session: &str,
        request: &PreparedRequest,
        now: u64,
    ) -> Result<OpaqueToken, SessionError> {
        let (session_hash, actor) = self.resolve(session, now).ok_or(SessionError::NoSession)?;
        let decision = policy.evaluate(Some(&actor), request, None);
        if decision.code != DecisionCode::AcknowledgementRequired {
            return Err(SessionError::NotPermitted);
        }
        if self.approvals.len() >= MAX_APPROVALS {
            return Err(SessionError::Capacity);
        }
        let token = fresh_token()?;
        let key = digest_secret(token.as_str()).ok_or(SessionError::Randomness)?;
        if self.approvals.contains_key(&key) {
            return Err(SessionError::Randomness);
        }
        let expires_at = now
            .checked_add(CONFIRM_TTL)
            .ok_or(SessionError::InvalidLifetime)?;
        self.approvals.insert(
            key,
            Approval {
                session_hash,
                request: request.clone(),
                expires_at,
            },
        );
        Ok(token)
    }

    /// This is the sole LAB-only path that combines live session state,
    /// one-shot confirmation and the existing fsync audit-backed permit.
    /// Any matching approval is consumed BEFORE attempting audit, so even
    /// an I/O error cannot make it reusable.
    pub fn grant_recorded_session(
        &mut self,
        policy: &Policy,
        raw_session: &str,
        request: &PreparedRequest,
        raw_approval: Option<&str>,
        journal: &mut FileAuditJournal,
        now: u64,
    ) -> io::Result<Option<AuditedPermit>> {
        let identity = self.resolve(raw_session, now);
        let mut acknowledgment = None;
        if let (Some((session_hash, actor)), Some(raw)) = (&identity, raw_approval)
            && let Some(approval_hash) = digest_secret(raw)
            && let Some(approval) = self.approvals.remove(&approval_hash)
            && approval.session_hash == *session_hash
            && now < approval.expires_at
            && approval.request == *request
        {
            acknowledgment = Some(Acknowledgement::bind(actor, request));
        }
        grant_recorded(
            policy,
            identity.as_ref().map(|(_, actor)| actor),
            request,
            acknowledgment.as_ref(),
            journal,
        )
    }
}
