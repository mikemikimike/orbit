//! Worker launches: durable records of launcher-managed worker instances.

use chrono::Utc;
use uuid::Uuid;

use crate::models::WorkerLaunch;
use crate::{Store, StoreError, StoreResult};

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub async fn create_worker_launch(
        &self,
        org_id: Uuid,
        profile_id: Uuid,
        name: &str,
        target: &str,
        image: &str,
        worker_token_id: Option<Uuid>,
        created_by: Option<Uuid>,
    ) -> StoreResult<WorkerLaunch> {
        sqlx::query_as::<_, WorkerLaunch>(
            r#"
            INSERT INTO worker_launches
                (id, org_id, profile_id, name, target, image, worker_token_id, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(orbit_domain::new_id())
        .bind(org_id)
        .bind(profile_id)
        .bind(name)
        .bind(target)
        .bind(image)
        .bind(worker_token_id)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_worker_launch(
        &self,
        org_id: Uuid,
        launch_id: Uuid,
    ) -> StoreResult<WorkerLaunch> {
        sqlx::query_as::<_, WorkerLaunch>(
            "SELECT * FROM worker_launches WHERE org_id = $1 AND id = $2",
        )
        .bind(org_id)
        .bind(launch_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StoreError::NotFound("worker launch".into()))
    }

    pub async fn list_worker_launches(&self, org_id: Uuid) -> StoreResult<Vec<WorkerLaunch>> {
        Ok(sqlx::query_as::<_, WorkerLaunch>(
            "SELECT * FROM worker_launches WHERE org_id = $1 ORDER BY created_at DESC LIMIT 100",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_worker_launch(
        &self,
        org_id: Uuid,
        launch_id: Uuid,
        status: &str,
        runtime_id: Option<&str>,
        worker_id: Option<&str>,
        error: Option<&str>,
        started: bool,
        stopped: bool,
    ) -> StoreResult<WorkerLaunch> {
        let started_at = started.then(Utc::now);
        let stopped_at = stopped.then(Utc::now);
        sqlx::query_as::<_, WorkerLaunch>(
            r#"
            UPDATE worker_launches
            SET status = $3,
                runtime_id = COALESCE($4, runtime_id),
                worker_id = COALESCE($5, worker_id),
                error = $6,
                started_at = COALESCE($7, started_at),
                stopped_at = COALESCE($8, stopped_at),
                updated_at = now()
            WHERE org_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(launch_id)
        .bind(status)
        .bind(runtime_id)
        .bind(worker_id)
        .bind(error)
        .bind(started_at)
        .bind(stopped_at)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StoreError::NotFound("worker launch".into()))
    }
}
