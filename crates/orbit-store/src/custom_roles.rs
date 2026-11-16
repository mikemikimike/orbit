//! Organization-defined roles with explicit permission sets.

use uuid::Uuid;

use crate::models::CustomRole;
use crate::{Store, StoreError, StoreResult};

impl Store {
    pub async fn list_custom_roles(&self, org_id: Uuid) -> StoreResult<Vec<CustomRole>> {
        Ok(sqlx::query_as::<_, CustomRole>(
            "SELECT * FROM custom_roles WHERE org_id = $1 ORDER BY name ASC",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_custom_role(&self, org_id: Uuid, role_id: Uuid) -> StoreResult<CustomRole> {
        sqlx::query_as::<_, CustomRole>("SELECT * FROM custom_roles WHERE org_id = $1 AND id = $2")
            .bind(org_id)
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| StoreError::NotFound("role".into()))
    }

    pub async fn create_custom_role(
        &self,
        org_id: Uuid,
        name: &str,
        permissions: &[String],
    ) -> StoreResult<CustomRole> {
        sqlx::query_as::<_, CustomRole>(
            r#"
            INSERT INTO custom_roles (id, org_id, name, permissions)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(org_id)
        .bind(name)
        .bind(permissions)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| {
            if error
                .as_database_error()
                .map(|db| db.is_unique_violation())
                .unwrap_or(false)
            {
                StoreError::Conflict(format!("role '{name}' already exists"))
            } else {
                error.into()
            }
        })
    }

    pub async fn update_custom_role(
        &self,
        org_id: Uuid,
        role_id: Uuid,
        name: &str,
        permissions: &[String],
    ) -> StoreResult<CustomRole> {
        sqlx::query_as::<_, CustomRole>(
            r#"
            UPDATE custom_roles
            SET name = $3, permissions = $4, updated_at = now()
            WHERE org_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(role_id)
        .bind(name)
        .bind(permissions)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StoreError::NotFound("role".into()))
    }

    pub async fn delete_custom_role(&self, org_id: Uuid, role_id: Uuid) -> StoreResult<()> {
        let result = sqlx::query("DELETE FROM custom_roles WHERE org_id = $1 AND id = $2")
            .bind(org_id)
            .bind(role_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound("role".into()));
        }
        Ok(())
    }

    pub async fn custom_role_permissions(
        &self,
        org_id: Uuid,
        name: &str,
    ) -> StoreResult<Option<Vec<String>>> {
        let row: Option<(Vec<String>,)> =
            sqlx::query_as("SELECT permissions FROM custom_roles WHERE org_id = $1 AND name = $2")
                .bind(org_id)
                .bind(name)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|(permissions,)| permissions))
    }

    pub async fn set_user_role_name(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> StoreResult<crate::models::User> {
        sqlx::query_as::<_, crate::models::User>(
            "UPDATE users SET role = $3, updated_at = now() WHERE org_id = $1 AND id = $2 RETURNING *",
        )
        .bind(org_id)
        .bind(user_id)
        .bind(role)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StoreError::NotFound("user".into()))
    }
}
