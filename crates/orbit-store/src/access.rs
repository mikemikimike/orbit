//! Repository-level visibility predicates (ADR 0011).

use uuid::Uuid;

/// Caller identity used by list queries. Store methods accept `Option<&ResourceAccess>`;
/// `None` means platform/internal access (workers, bootstrap, tests).
#[derive(Debug, Clone)]
pub struct ResourceAccess {
    pub user_id: Uuid,
    /// Org owner/admin: sees every resource.
    pub privileged: bool,
    /// Access-team memberships. Leads are included.
    pub team_ids: Vec<Uuid>,
}

impl ResourceAccess {
    pub fn new(user_id: Uuid, privileged: bool, team_ids: Vec<Uuid>) -> Self {
        Self {
            user_id,
            privileged,
            team_ids,
        }
    }
}

/// Bind values for the standard predicate. Every query uses the same shape:
/// `($n::bool OR alias.visibility = 'org' OR alias.owner_user_id = $n+1
///   OR (alias.visibility = 'team' AND alias.owner_team_id = ANY($n+2)))`
pub(crate) fn binds(access: Option<&ResourceAccess>) -> (bool, Uuid, Vec<Uuid>) {
    match access {
        Some(access) => (access.privileged, access.user_id, access.team_ids.clone()),
        None => (true, Uuid::nil(), Vec::new()),
    }
}
