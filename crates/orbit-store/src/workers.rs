use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::models::ExecutionRecord;
use crate::{Store, StoreResult};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WorkerHeartbeat {
    pub worker_id: String,
    pub runtimes: Vec<String>,
    pub hostname: Option<String>,
    pub started_at: DateTime<Utc>,
    pub last_heartbeat_at: DateTime<Utc>,
}

impl Store {
    pub async fn upsert_worker_heartbeat(
        &self,
        worker_id: &str,
        runtimes: &[String],
        hostname: Option<&str>,
    ) -> StoreResult<()> {
        sqlx::query(
            r#"
            INSERT INTO worker_heartbeats (worker_id, runtimes, hostname, last_heartbeat_at)
            VALUES ($1, $2, $3, now())
            ON CONFLICT (worker_id) DO UPDATE SET
                runtimes = EXCLUDED.runtimes,
                hostname = COALESCE(EXCLUDED.hostname, worker_heartbeats.hostname),
                last_heartbeat_at = now()
            "#,
        )
        .bind(worker_id)
        .bind(runtimes)
        .bind(hostname)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_worker_heartbeat(&self, worker_id: &str) -> StoreResult<()> {
        sqlx::query("DELETE FROM worker_heartbeats WHERE worker_id = $1")
            .bind(worker_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn worker_heartbeat_age_seconds(&self, worker_id: &str) -> StoreResult<Option<i64>> {
        Ok(sqlx::query_scalar(
            "SELECT EXTRACT(EPOCH FROM (now() - last_heartbeat_at))::bigint FROM worker_heartbeats WHERE worker_id = $1",
        )
        .bind(worker_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Mark executions whose worker heartbeat expired as failed and expire their pending
    /// approvals. Returns the affected executions so the caller can emit events.
    ///
    /// This is deliberately conservative: orphaned work is failed, not re-run, to avoid
    /// duplicate side effects. Retries become a separate, idempotency-aware feature.
    pub async fn reap_orphaned_executions(
        &self,
        stale_after_seconds: i64,
    ) -> StoreResult<Vec<ExecutionRecord>> {
        self.reap_orphaned_executions_scoped(None, stale_after_seconds).await
    }

    /// Org-scoped reaper used by tests and future per-tenant maintenance jobs.
    pub async fn reap_orphaned_executions_for_org(
        &self,
        org_id: Uuid,
        stale_after_seconds: i64,
    ) -> StoreResult<Vec<ExecutionRecord>> {
        self.reap_orphaned_executions_scoped(Some(org_id), stale_after_seconds)
            .await
    }

    async fn reap_orphaned_executions_scoped(
        &self,
        org_id: Option<Uuid>,
        stale_after_seconds: i64,
    ) -> StoreResult<Vec<ExecutionRecord>> {
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query_as::<_, ExecutionRecord>(
            r#"
            UPDATE executions e
            SET status = 'failed',
                error = jsonb_build_object(
                    'code', 'worker_lost',
                    'message', 'worker heartbeat expired; execution outcome is unknown',
                    'retryable', true
                ),
                finished_at = now(),
                updated_at = now()
            WHERE e.status IN ('dispatched', 'running', 'awaiting_approval')
              AND e.worker_id IS NOT NULL
              -- Grace period: do not reap work that was just claimed while a worker is
              -- still starting up and has not written its first heartbeat yet.
              AND e.created_at < now() - interval '15 seconds'
              AND ($2::uuid IS NULL OR e.org_id = $2)
              AND NOT EXISTS (
                  SELECT 1 FROM worker_heartbeats h
                  WHERE h.worker_id = e.worker_id
                    AND h.last_heartbeat_at > now() - make_interval(secs => $1)
              )
            RETURNING e.*
            "#,
        )
        .bind(stale_after_seconds as f64)
        .bind(org_id)
        .fetch_all(&mut *tx)
        .await?;

        if !rows.is_empty() {
            let ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
            sqlx::query(
                r#"
                UPDATE approvals
                SET status = 'expired', decided_at = now()
                WHERE execution_id = ANY($1) AND status = 'pending'
                "#,
            )
            .bind(&ids)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(rows)
    }

    pub async fn list_worker_heartbeats(&self) -> StoreResult<Vec<WorkerHeartbeat>> {
        Ok(sqlx::query_as::<_, WorkerHeartbeat>(
            "SELECT * FROM worker_heartbeats ORDER BY last_heartbeat_at DESC",
        )
        .fetch_all(&self.pool)
        .await?)
    }
}

/// JSON payload used in execution error records for orphaned work.
pub fn worker_lost_error() -> serde_json::Value {
    json!({
        "code": "worker_lost",
        "message": "worker heartbeat expired; execution outcome is unknown",
        "retryable": true
    })
}
