use serde_json::Value;
use uuid::Uuid;

use orbit_security::{AuthContext, AuthMethod};

use crate::state::AppState;

/// Best-effort audit write. Never fails a user request, but logs loudly.
pub async fn record(
    state: &AppState,
    auth: &AuthContext,
    action: &str,
    target_type: Option<&str>,
    target_id: Option<Uuid>,
    metadata: Value,
) {
    let actor_type = match auth.method {
        AuthMethod::ApiToken => "api_token",
        AuthMethod::Anonymous | AuthMethod::Dev | AuthMethod::Oidc => "user",
    };
    let result = state
        .store
        .append_audit_log(
            auth.org_id,
            actor_type,
            Some(auth.user_id),
            Some(&auth.email),
            action,
            target_type,
            target_id,
            metadata,
            Some(&crate::request_id::current()),
            None,
        )
        .await;
    if let Err(err) = result {
        tracing::error!(error = %err, action, "failed to write audit log");
    }
}
