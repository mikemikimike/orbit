//! Tool policy persistence.

use uuid::Uuid;

use orbit_domain::{ToolPolicyAction, ToolPolicyMatcher, ToolPolicyRule};

use crate::models::ToolPolicy;
use crate::{Store, StoreError, StoreResult};

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub async fn create_tool_policy(
        &self,
        org_id: Uuid,
        agent_id: Option<Uuid>,
        name: &str,
        priority: i32,
        enabled: bool,
        action: &str,
        matcher: &serde_json::Value,
        reason: Option<&str>,
        impact: Option<&str>,
    ) -> StoreResult<ToolPolicy> {
        sqlx::query_as::<_, ToolPolicy>(
            r#"
            INSERT INTO tool_policies
                (id, org_id, agent_id, name, priority, enabled, action, matcher, reason, impact)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(org_id)
        .bind(agent_id)
        .bind(name)
        .bind(priority)
        .bind(enabled)
        .bind(action)
        .bind(matcher)
        .bind(reason)
        .bind(impact)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| {
            if error
                .as_database_error()
                .map(|db| db.is_unique_violation())
                .unwrap_or(false)
            {
                StoreError::Conflict(format!("tool policy '{name}' already exists"))
            } else {
                error.into()
            }
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_tool_policy(
        &self,
        org_id: Uuid,
        policy_id: Uuid,
        agent_id: Option<Uuid>,
        name: &str,
        priority: i32,
        enabled: bool,
        action: &str,
        matcher: &serde_json::Value,
        reason: Option<&str>,
        impact: Option<&str>,
    ) -> StoreResult<ToolPolicy> {
        sqlx::query_as::<_, ToolPolicy>(
            r#"
            UPDATE tool_policies
            SET agent_id = $3, name = $4, priority = $5, enabled = $6, action = $7,
                matcher = $8, reason = $9, impact = $10, updated_at = now()
            WHERE org_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(policy_id)
        .bind(agent_id)
        .bind(name)
        .bind(priority)
        .bind(enabled)
        .bind(action)
        .bind(matcher)
        .bind(reason)
        .bind(impact)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StoreError::NotFound("tool policy".into()))
    }

    pub async fn delete_tool_policy(&self, org_id: Uuid, policy_id: Uuid) -> StoreResult<()> {
        let result = sqlx::query("DELETE FROM tool_policies WHERE org_id = $1 AND id = $2")
            .bind(org_id)
            .bind(policy_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound("tool policy".into()));
        }
        Ok(())
    }

    /// List rules for management: without an agent this is the org-level set; with an
    /// agent it is the effective set (org defaults + that agent's rules).
    pub async fn list_tool_policies(
        &self,
        org_id: Uuid,
        agent_id: Option<Uuid>,
    ) -> StoreResult<Vec<ToolPolicy>> {
        let rows = match agent_id {
            Some(agent_id) => {
                sqlx::query_as::<_, ToolPolicy>(
                    r#"
                    SELECT * FROM tool_policies
                    WHERE org_id = $1 AND (agent_id IS NULL OR agent_id = $2)
                    ORDER BY priority ASC, name ASC
                    "#,
                )
                .bind(org_id)
                .bind(agent_id)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, ToolPolicy>(
                    r#"
                    SELECT * FROM tool_policies
                    WHERE org_id = $1 AND agent_id IS NULL
                    ORDER BY priority ASC, name ASC
                    "#,
                )
                .bind(org_id)
                .fetch_all(&self.pool)
                .await?
            }
        };
        Ok(rows)
    }

    /// Effective rules for an execution, ready for the worker/broker. Rows whose stored
    /// action or matcher cannot be parsed are skipped (fail open to the platform decision;
    /// invalid rules should not be created through the API).
    pub async fn effective_tool_policy_rules(
        &self,
        org_id: Uuid,
        agent_id: Uuid,
    ) -> StoreResult<Vec<ToolPolicyRule>> {
        let rows = self.list_tool_policies(org_id, Some(agent_id)).await?;
        Ok(rows.into_iter().filter_map(policy_into_rule).collect())
    }
}

fn policy_into_rule(policy: ToolPolicy) -> Option<ToolPolicyRule> {
    let action = ToolPolicyAction::parse(&policy.action)?;
    let matcher: ToolPolicyMatcher = serde_json::from_value(policy.matcher).ok()?;
    Some(ToolPolicyRule {
        id: policy.id,
        name: policy.name,
        priority: policy.priority,
        enabled: policy.enabled,
        action,
        matcher,
        reason: policy.reason,
        impact: policy.impact,
    })
}
