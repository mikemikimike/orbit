use uuid::Uuid;

use orbit_security::{SecretStore, SecretValue};

use crate::{Store, StoreError, StoreResult};

impl Store {
    /// Store (or replace) an encrypted secret. Secrets are always encrypted at rest:
    /// a store without a configured cipher refuses writes.
    pub async fn put_secret(
        &self,
        org_id: Option<Uuid>,
        reference: &str,
        value: &SecretValue,
        description: &str,
    ) -> StoreResult<()> {
        let ciphertext = self.cipher()?.encrypt(reference, value)?;
        sqlx::query(
            r#"
            INSERT INTO secrets (reference, org_id, ciphertext, description)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (reference)
            DO UPDATE SET ciphertext = EXCLUDED.ciphertext,
                          org_id = EXCLUDED.org_id,
                          description = EXCLUDED.description,
                          updated_at = now()
            "#,
        )
        .bind(reference)
        .bind(org_id)
        .bind(ciphertext)
        .bind(description)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_secret_value(&self, reference: &str) -> StoreResult<Option<SecretValue>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT ciphertext FROM secrets WHERE reference = $1")
                .bind(reference)
                .fetch_optional(&self.pool)
                .await?;
        match row {
            None => Ok(None),
            Some((ciphertext,)) => {
                let value = self
                    .cipher()?
                    .decrypt(reference, &ciphertext)
                    .map_err(|e| StoreError::Internal(e.to_string()))?;
                Ok(Some(value))
            }
        }
    }

    pub async fn delete_secret(&self, reference: &str) -> StoreResult<()> {
        sqlx::query("DELETE FROM secrets WHERE reference = $1")
            .bind(reference)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl SecretStore for Store {
    async fn put(
        &self,
        reference: &str,
        value: SecretValue,
    ) -> Result<(), orbit_security::SecurityError> {
        self.put_secret(None, reference, &value, "")
            .await
            .map_err(|e| orbit_security::SecurityError::Secret(e.to_string()))
    }

    async fn get(
        &self,
        reference: &str,
    ) -> Result<Option<SecretValue>, orbit_security::SecurityError> {
        self.get_secret_value(reference)
            .await
            .map_err(|e| orbit_security::SecurityError::Secret(e.to_string()))
    }

    async fn delete(&self, reference: &str) -> Result<(), orbit_security::SecurityError> {
        self.delete_secret(reference)
            .await
            .map_err(|e| orbit_security::SecurityError::Secret(e.to_string()))
    }
}
