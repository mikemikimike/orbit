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
    /// Strict team isolation: a team-owned resource is visible only to members of its
    /// owning team, whatever its visibility (mirrors `AccessScope::can_view` in the API).
    pub team_isolation: bool,
}

impl ResourceAccess {
    pub fn new(user_id: Uuid, privileged: bool, team_ids: Vec<Uuid>) -> Self {
        Self {
            user_id,
            privileged,
            team_ids,
            team_isolation: false,
        }
    }

    pub fn with_team_isolation(mut self, enabled: bool) -> Self {
        self.team_isolation = enabled;
        self
    }
}

/// Bind values for the standard predicate. Every query uses the same shape:
/// `($p::bool OR alias.owner_user_id = $u
///   OR CASE WHEN $i::bool AND alias.owner_team_id IS NOT NULL
///        THEN alias.owner_team_id = ANY($t)
///        ELSE alias.visibility = 'org'
///             OR (alias.visibility = 'team' AND alias.owner_team_id = ANY($t)) END)`
/// with `$p, $u, $t, $i` = privileged, user id, team ids, team isolation.
pub(crate) fn binds(access: Option<&ResourceAccess>) -> (bool, Uuid, Vec<Uuid>, bool) {
    match access {
        Some(access) => (
            access.privileged,
            access.user_id,
            access.team_ids.clone(),
            access.team_isolation,
        ),
        None => (true, Uuid::nil(), Vec::new(), false),
    }
}
