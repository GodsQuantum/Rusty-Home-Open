# Fresh Core admin check for issue and consumption of approval (lab only)

One-time, request-bound approval is now guarded on BOTH sides:
the challenge is issued only after accepted Supervisor Ingress TCP-peer checks,
a fresh authenticated Core WebSocket query, positive current human administrator
role, and identity equality with the unexpired, private durable fixture session.
An arbitrary App or action without an explicit grant cannot request a challenge.

The same independent Core admin check is repeated when consuming an approval;
disabling/demoting the Core user after issuance blocks the permit. The existing
ACID redb store rejects replay, mismatched App, action and backup ID, expiry
and revocation. The hash-chained fsynced audit remains mandatory before minting
any preview capability.

This feature is not enabled in the default product and has no POST route.
The fixture session issuer still lacks a trusted human login UI and an
independent production identity provider; CSRF, origin checks, secure
time, permissions administration, backup/rollback and hardware tests remain
blocking. A proof about existing components is not a 15/15 release signoff.
