//! Platform-wide settings (key/value JSONB).

use serde_json::Value;
use uuid::Uuid;

use crate::{Store, StoreResult};

impl Store {
    pub async fn get_platform_setting(&self, key: &str) -> StoreResult<Option<Value>> {
        Ok(sqlx::query_scalar::<_, Value>(
            "SELECT value FROM platform_settings WHERE key = $1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn set_platform_setting(
        &self,
        key: &str,
        value: &Value,
        updated_by: Option<Uuid>,
    ) -> StoreResult<Value> {
        sqlx::query_scalar::<_, Value>(
            r#"
            INSERT INTO platform_settings (key, value, updated_by)
            VALUES ($1, $2, $3)
            ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_by = EXCLUDED.updated_by, updated_at = now()
            RETURNING value
            "#,
        )
        .bind(key)
        .bind(value)
        .bind(updated_by)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }
}
