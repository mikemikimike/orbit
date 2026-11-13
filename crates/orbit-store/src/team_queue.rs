//! Durable Agent Team work queue.

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::{Store, StoreError, StoreResult};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TeamQueueItem {
    pub id: Uuid,
    pub org_id: Uuid,
    pub team_id: Uuid,
    pub external_key: String,
    pub repository: Option<String>,
    pub payload: Value,
    pub status: String,
    pub team_run_id: Option<Uuid>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub attempts: i32,
    pub max_attempts: i32,
}

impl Store {
    pub async fn list_team_queue(
        &self,
        org_id: Uuid,
        team_id: Uuid,
    ) -> StoreResult<Vec<TeamQueueItem>> {
        Ok(sqlx::query_as::<_, TeamQueueItem>(
            "SELECT * FROM team_queue_items WHERE org_id = $1 AND team_id = $2 ORDER BY created_at ASC",
        )
        .bind(org_id)
        .bind(team_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn enqueue_team_item(
        &self,
        org_id: Uuid,
        team_id: Uuid,
        external_key: &str,
        repository: Option<&str>,
        payload: &Value,
    ) -> StoreResult<TeamQueueItem> {
        sqlx::query_as::<_, TeamQueueItem>(
            r#"
            INSERT INTO team_queue_items (id, org_id, team_id, external_key, repository, payload)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (team_id, external_key) DO UPDATE SET updated_at = now()
            RETURNING *
            "#,
        )
        .bind(orbit_domain::new_id())
        .bind(org_id)
        .bind(team_id)
        .bind(external_key)
        .bind(repository)
        .bind(payload)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    /// Insert a work-source item only when the dedupe key is not already present.
    /// Returns `None` when the item already exists, so callers can skip cleanly.
    #[allow(clippy::too_many_arguments)]
    pub async fn try_enqueue_team_item(
        &self,
        org_id: Uuid,
        team_id: Uuid,
        external_key: &str,
        repository: Option<&str>,
        payload: &Value,
        max_attempts: i32,
    ) -> StoreResult<Option<TeamQueueItem>> {
        sqlx::query_as::<_, TeamQueueItem>(
            r#"
            INSERT INTO team_queue_items
                (id, org_id, team_id, external_key, repository, payload, max_attempts)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (team_id, external_key) DO NOTHING
            RETURNING *
            "#,
        )
        .bind(orbit_domain::new_id())
        .bind(org_id)
        .bind(team_id)
        .bind(external_key)
        .bind(repository)
        .bind(payload)
        .bind(max_attempts)
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn list_team_queue_external_keys(&self, team_id: Uuid) -> StoreResult<Vec<String>> {
        Ok(sqlx::query_scalar::<_, String>(
            "SELECT external_key FROM team_queue_items WHERE team_id = $1",
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_team_queue_item_by_run(
        &self,
        team_run_id: Uuid,
    ) -> StoreResult<Option<TeamQueueItem>> {
        Ok(sqlx::query_as::<_, TeamQueueItem>(
            "SELECT * FROM team_queue_items WHERE team_run_id = $1",
        )
        .bind(team_run_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn get_team_queue_item(
        &self,
        org_id: Uuid,
        item_id: Uuid,
    ) -> StoreResult<TeamQueueItem> {
        sqlx::query_as::<_, TeamQueueItem>(
            "SELECT * FROM team_queue_items WHERE org_id = $1 AND id = $2",
        )
        .bind(org_id)
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StoreError::NotFound("team queue item".into()))
    }

    pub async fn set_team_queue_run(
        &self,
        org_id: Uuid,
        item_id: Uuid,
        team_run_id: Uuid,
        status: &str,
    ) -> StoreResult<()> {
        sqlx::query(
            "UPDATE team_queue_items SET team_run_id = $3, status = $4, updated_at = now() WHERE org_id = $1 AND id = $2",
        )
        .bind(org_id)
        .bind(item_id)
        .bind(team_run_id)
        .bind(status)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_team_queue_status_by_run(
        &self,
        team_run_id: Uuid,
        status: &str,
        error: Option<&str>,
    ) -> StoreResult<()> {
        sqlx::query(
            "UPDATE team_queue_items SET status = $2, error = $3, updated_at = now() WHERE team_run_id = $1",
        )
        .bind(team_run_id)
        .bind(status)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_team_queue_status(
        &self,
        org_id: Uuid,
        item_id: Uuid,
        status: &str,
        error: Option<&str>,
    ) -> StoreResult<()> {
        let result = sqlx::query(
            "UPDATE team_queue_items SET status = $3, error = $4, updated_at = now() WHERE org_id = $1 AND id = $2",
        )
        .bind(org_id)
        .bind(item_id)
        .bind(status)
        .bind(error)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound("team queue item".into()));
        }
        Ok(())
    }

    /// Return a failed item to pending for another attempt.
    pub async fn retry_team_queue_item(
        &self,
        org_id: Uuid,
        item_id: Uuid,
        error: Option<&str>,
    ) -> StoreResult<TeamQueueItem> {
        sqlx::query_as::<_, TeamQueueItem>(
            r#"
            UPDATE team_queue_items
            SET status = 'pending', error = $3, attempts = attempts + 1,
                team_run_id = NULL, updated_at = now()
            WHERE org_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(item_id)
        .bind(error)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StoreError::NotFound("team queue item".into()))
    }

    /// Manual requeue: pending again without consuming an attempt.
    pub async fn requeue_team_queue_item(
        &self,
        org_id: Uuid,
        item_id: Uuid,
    ) -> StoreResult<TeamQueueItem> {
        sqlx::query_as::<_, TeamQueueItem>(
            r#"
            UPDATE team_queue_items
            SET status = 'pending', error = NULL, team_run_id = NULL, updated_at = now()
            WHERE org_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StoreError::NotFound("team queue item".into()))
    }

    /// Cancel a queued/running item and detach any in-flight run link.
    pub async fn cancel_team_queue_item(
        &self,
        org_id: Uuid,
        item_id: Uuid,
        error: Option<&str>,
    ) -> StoreResult<TeamQueueItem> {
        sqlx::query_as::<_, TeamQueueItem>(
            r#"
            UPDATE team_queue_items
            SET status = 'cancelled', error = $3, team_run_id = NULL, updated_at = now()
            WHERE org_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(item_id)
        .bind(error)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StoreError::NotFound("team queue item".into()))
    }
}
