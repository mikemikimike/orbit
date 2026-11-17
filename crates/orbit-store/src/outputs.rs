//! Execution result outputs and their durable delivery queue.

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::models::{ExecutionRecord, Output, OutputDeliveryJob, TeamRun};
use crate::{Store, StoreError, StoreResult};

impl Store {
    pub async fn list_outputs(&self, org_id: Uuid) -> StoreResult<Vec<Output>> {
        Ok(
            sqlx::query_as::<_, Output>(
                "SELECT * FROM outputs WHERE org_id = $1 ORDER BY name ASC",
            )
            .bind(org_id)
            .fetch_all(&self.pool)
            .await?,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_output(
        &self,
        org_id: Uuid,
        name: &str,
        kind: &str,
        agent_id: Option<Uuid>,
        team_id: Option<Uuid>,
        events: &[String],
        secret_ref: &str,
        template: Option<&str>,
    ) -> StoreResult<Output> {
        sqlx::query_as::<_, Output>(
            r#"
            INSERT INTO outputs (id, org_id, name, kind, agent_id, team_id, events, secret_ref, template)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(org_id)
        .bind(name)
        .bind(kind)
        .bind(agent_id)
        .bind(team_id)
        .bind(events)
        .bind(secret_ref)
        .bind(template)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| {
            if error
                .as_database_error()
                .map(|db| db.is_unique_violation())
                .unwrap_or(false)
            {
                StoreError::Conflict(format!("output '{name}' already exists"))
            } else {
                error.into()
            }
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_output(
        &self,
        org_id: Uuid,
        id: Uuid,
        name: &str,
        kind: &str,
        agent_id: Option<Uuid>,
        team_id: Option<Uuid>,
        events: &[String],
        secret_ref: Option<&str>,
        template: Option<&str>,
        enabled: bool,
    ) -> StoreResult<Output> {
        sqlx::query_as::<_, Output>(
            r#"
            UPDATE outputs
            SET name = $3, kind = $4, agent_id = $5, team_id = $6, events = $7,
                secret_ref = COALESCE($8, secret_ref), template = $9, enabled = $10,
                updated_at = now()
            WHERE org_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(id)
        .bind(name)
        .bind(kind)
        .bind(agent_id)
        .bind(team_id)
        .bind(events)
        .bind(secret_ref)
        .bind(template)
        .bind(enabled)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StoreError::NotFound("output".into()))
    }

    pub async fn delete_output(&self, org_id: Uuid, id: Uuid) -> StoreResult<()> {
        let result = sqlx::query("DELETE FROM outputs WHERE org_id = $1 AND id = $2")
            .bind(org_id)
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound("output".into()));
        }
        Ok(())
    }

    pub async fn get_output(&self, org_id: Uuid, id: Uuid) -> StoreResult<Output> {
        sqlx::query_as::<_, Output>("SELECT * FROM outputs WHERE org_id = $1 AND id = $2")
            .bind(org_id)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| StoreError::NotFound("output".into()))
    }

    /// Enqueue deliveries for recent terminal executions. Idempotent per output/execution.
    pub async fn enqueue_output_deliveries(&self, since: DateTime<Utc>) -> StoreResult<u64> {
        let result = sqlx::query(
            r#"
            INSERT INTO output_deliveries (id, org_id, output_id, execution_id)
            SELECT gen_random_uuid(), o.org_id, o.id, e.id
            FROM outputs o
            JOIN executions e
              ON e.org_id = o.org_id
             AND (o.agent_id IS NULL OR o.agent_id = e.agent_id)
             AND e.status = ANY(o.events)
             AND e.updated_at >= $1
            WHERE o.enabled AND o.team_id IS NULL
            ON CONFLICT (output_id, execution_id) DO NOTHING
            "#,
        )
        .bind(since)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Enqueue team-run outputs. Global/agent outputs keep executing per execution.
    pub async fn enqueue_team_output_deliveries(&self, since: DateTime<Utc>) -> StoreResult<u64> {
        let result = sqlx::query(
            r#"
            INSERT INTO output_deliveries (id, org_id, output_id, team_run_id)
            SELECT gen_random_uuid(), o.org_id, o.id, t.id
            FROM outputs o
            JOIN team_runs t
              ON t.org_id = o.org_id
             AND o.team_id = t.team_id
             AND t.status = ANY(o.events)
             AND t.updated_at >= $1
            WHERE o.enabled AND o.team_id IS NOT NULL
            ON CONFLICT (output_id, team_run_id) DO NOTHING
            "#,
        )
        .bind(since)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn due_output_deliveries(&self, limit: i64) -> StoreResult<Vec<OutputDeliveryJob>> {
        Ok(sqlx::query_as::<_, OutputDeliveryJob>(
            r#"
            SELECT id, org_id, output_id, execution_id, team_run_id, attempts
            FROM output_deliveries
            WHERE status = 'pending' AND next_attempt_at <= now()
            ORDER BY next_attempt_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_team_run_any_org(&self, id: Uuid) -> StoreResult<TeamRun> {
        sqlx::query_as::<_, TeamRun>("SELECT * FROM team_runs WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| StoreError::NotFound("team run".into()))
    }

    pub async fn get_execution_by_id_any_org(&self, id: Uuid) -> StoreResult<ExecutionRecord> {
        sqlx::query_as::<_, ExecutionRecord>("SELECT * FROM executions WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| StoreError::NotFound("execution".into()))
    }

    pub async fn finish_output_delivery(
        &self,
        id: Uuid,
        success: bool,
        response_status: Option<i32>,
        error: Option<&str>,
        attempts: i32,
    ) -> StoreResult<()> {
        let (status, next_attempt) = if success {
            ("succeeded", Utc::now())
        } else if attempts >= 5 {
            ("dead", Utc::now())
        } else {
            (
                "pending",
                Utc::now() + Duration::seconds(30 * i64::from(attempts.max(1))),
            )
        };
        sqlx::query(
            r#"
            UPDATE output_deliveries
            SET status = $2, attempts = $3, next_attempt_at = $4,
                response_status = $5, last_error = $6, updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(attempts)
        .bind(next_attempt)
        .bind(response_status)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
