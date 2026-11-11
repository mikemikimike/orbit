//! Agent teams and sequential team runs.

use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use crate::models::{AgentTeam, TeamRun, TeamStep};
use crate::{Store, StoreError, StoreResult};

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub async fn create_agent_team(
        &self,
        org_id: Uuid,
        name: &str,
        description: &str,
        definition: &Value,
        created_by: Option<Uuid>,
    ) -> StoreResult<AgentTeam> {
        sqlx::query_as::<_, AgentTeam>(
            r#"
            INSERT INTO agent_teams (id, org_id, name, description, definition, created_by)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(orbit_domain::new_id())
        .bind(org_id)
        .bind(name)
        .bind(description)
        .bind(definition)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn list_agent_teams(&self, org_id: Uuid) -> StoreResult<Vec<AgentTeam>> {
        Ok(sqlx::query_as::<_, AgentTeam>(
            "SELECT * FROM agent_teams WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_agent_team(&self, org_id: Uuid, team_id: Uuid) -> StoreResult<AgentTeam> {
        sqlx::query_as::<_, AgentTeam>("SELECT * FROM agent_teams WHERE org_id = $1 AND id = $2")
            .bind(org_id)
            .bind(team_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| StoreError::NotFound("agent team".into()))
    }

    pub async fn delete_agent_team(&self, org_id: Uuid, team_id: Uuid) -> StoreResult<()> {
        let result = sqlx::query("DELETE FROM agent_teams WHERE org_id = $1 AND id = $2")
            .bind(org_id)
            .bind(team_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound("agent team".into()));
        }
        Ok(())
    }

    pub async fn create_team_run(
        &self,
        org_id: Uuid,
        team_id: Uuid,
        input: &Value,
        requested_by: Option<Uuid>,
    ) -> StoreResult<TeamRun> {
        sqlx::query_as::<_, TeamRun>(
            r#"
            INSERT INTO team_runs (id, org_id, team_id, status, input, requested_by)
            VALUES ($1, $2, $3, 'running', $4, $5)
            RETURNING *
            "#,
        )
        .bind(orbit_domain::new_id())
        .bind(org_id)
        .bind(team_id)
        .bind(input)
        .bind(requested_by)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_team_run(&self, org_id: Uuid, run_id: Uuid) -> StoreResult<TeamRun> {
        sqlx::query_as::<_, TeamRun>("SELECT * FROM team_runs WHERE org_id = $1 AND id = $2")
            .bind(org_id)
            .bind(run_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| StoreError::NotFound("team run".into()))
    }

    pub async fn list_team_runs(&self, org_id: Uuid) -> StoreResult<Vec<TeamRun>> {
        Ok(sqlx::query_as::<_, TeamRun>(
            "SELECT * FROM team_runs WHERE org_id = $1 ORDER BY created_at DESC LIMIT 100",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn list_running_team_runs(&self) -> StoreResult<Vec<TeamRun>> {
        Ok(sqlx::query_as::<_, TeamRun>(
            "SELECT * FROM team_runs WHERE status = 'running' ORDER BY created_at ASC LIMIT 50",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn update_team_run(
        &self,
        run_id: Uuid,
        status: &str,
        output: Option<&Value>,
        error: Option<&str>,
    ) -> StoreResult<TeamRun> {
        let finished = matches!(status, "succeeded" | "failed" | "cancelled");
        sqlx::query_as::<_, TeamRun>(
            r#"
            UPDATE team_runs
            SET status = $2, output = COALESCE($3, output), error = $4,
                finished_at = CASE WHEN $5 THEN now() ELSE finished_at END,
                updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(run_id)
        .bind(status)
        .bind(output)
        .bind(error)
        .bind(finished)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_team_step(
        &self,
        run_id: Uuid,
        org_id: Uuid,
        step_index: i32,
        name: &str,
        agent_id: Uuid,
        agent_version_id: Uuid,
        skills: &[String],
        input: &Value,
        execution_id: Option<Uuid>,
    ) -> StoreResult<TeamStep> {
        sqlx::query_as::<_, TeamStep>(
            r#"
            INSERT INTO team_steps
                (id, run_id, org_id, step_index, name, agent_id, agent_version_id, skills,
                 status, input, execution_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'running', $9, $10)
            RETURNING *
            "#,
        )
        .bind(orbit_domain::new_id())
        .bind(run_id)
        .bind(org_id)
        .bind(step_index)
        .bind(name)
        .bind(agent_id)
        .bind(agent_version_id)
        .bind(skills)
        .bind(input)
        .bind(execution_id)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn list_team_steps(&self, run_id: Uuid) -> StoreResult<Vec<TeamStep>> {
        Ok(sqlx::query_as::<_, TeamStep>(
            "SELECT * FROM team_steps WHERE run_id = $1 ORDER BY step_index ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn current_team_step(&self, run_id: Uuid) -> StoreResult<Option<TeamStep>> {
        Ok(sqlx::query_as::<_, TeamStep>(
            "SELECT * FROM team_steps WHERE run_id = $1 ORDER BY step_index DESC LIMIT 1",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn finish_team_step(
        &self,
        step_id: Uuid,
        status: &str,
        output: Option<&Value>,
        error: Option<&str>,
    ) -> StoreResult<TeamStep> {
        sqlx::query_as::<_, TeamStep>(
            r#"
            UPDATE team_steps
            SET status = $2, output = $3, error = $4,
                finished_at = now(), updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(step_id)
        .bind(status)
        .bind(output)
        .bind(error)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn set_team_step_execution(
        &self,
        step_id: Uuid,
        execution_id: Uuid,
    ) -> StoreResult<TeamStep> {
        sqlx::query_as::<_, TeamStep>(
            r#"
            UPDATE team_steps
            SET execution_id = $2, started_at = COALESCE(started_at, now()), updated_at = now()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(step_id)
        .bind(execution_id)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }
}

#[allow(dead_code)]
fn _value(_: Value, _: Utc) {}
