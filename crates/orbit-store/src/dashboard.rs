use serde_json::json;
use uuid::Uuid;

use crate::models::{DashboardSummary, ExecutionRecord};
use crate::{Store, StoreResult};

impl Store {
    pub async fn dashboard_summary(&self, org_id: Uuid) -> StoreResult<DashboardSummary> {
        let (total_agents, active_agents, paused_agents, draft_agents): (i64, i64, i64, i64) =
            sqlx::query_as(
                r#"
                SELECT
                    COUNT(*) FILTER (WHERE status <> 'archived'),
                    COUNT(*) FILTER (WHERE status = 'active'),
                    COUNT(*) FILTER (WHERE status = 'paused'),
                    COUNT(*) FILTER (WHERE status = 'draft')
                FROM agents WHERE org_id = $1
                "#,
            )
            .bind(org_id)
            .fetch_one(&self.pool)
            .await?;

        let (executions_24h, failed_executions_24h, running_executions): (i64, i64, i64) =
            sqlx::query_as(
                r#"
                SELECT
                    COUNT(*) FILTER (WHERE created_at > now() - interval '24 hours'),
                    COUNT(*) FILTER (WHERE created_at > now() - interval '24 hours'
                                      AND status IN ('failed', 'timed_out')),
                    COUNT(*) FILTER (WHERE status IN ('queued', 'dispatched', 'running'))
                FROM executions WHERE org_id = $1
                "#,
            )
            .bind(org_id)
            .fetch_one(&self.pool)
            .await?;

        let total_connections: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM connections WHERE org_id = $1 AND status = 'active'",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        let enabled_triggers: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM triggers WHERE org_id = $1 AND enabled")
                .bind(org_id)
                .fetch_one(&self.pool)
                .await?;

        let pending_approvals: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM approvals WHERE org_id = $1 AND status = 'pending'",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        let recent_executions = sqlx::query_as::<_, ExecutionRecord>(
            "SELECT * FROM executions WHERE org_id = $1 ORDER BY created_at DESC LIMIT 8",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(DashboardSummary {
            total_agents,
            active_agents,
            paused_agents,
            draft_agents,
            executions_24h,
            failed_executions_24h,
            running_executions,
            total_connections,
            enabled_triggers,
            recent_executions,
            pending_approvals,
        })
    }

    /// Lightweight usage rollup used by the dashboard and future quota enforcement.
    pub async fn usage_rollup(&self, org_id: Uuid, days: i64) -> StoreResult<serde_json::Value> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT to_char(date_trunc('day', created_at), 'YYYY-MM-DD') AS day, COUNT(*)
            FROM executions
            WHERE org_id = $1 AND created_at > now() - ($2 || ' days')::interval
            GROUP BY 1 ORDER BY 1
            "#,
        )
        .bind(org_id)
        .bind(days.to_string())
        .fetch_all(&self.pool)
        .await?;
        Ok(json!({
            "days": rows.into_iter().map(|(day, count)| json!({ "day": day, "executions": count })).collect::<Vec<_>>()
        }))
    }
}
