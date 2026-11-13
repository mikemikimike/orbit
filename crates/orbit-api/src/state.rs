use std::sync::Arc;

use orbit_events::EventBus;
use orbit_harness::HarnessRegistry;
use orbit_security::{AuthConfig, JwtValidator};
use orbit_store::Store;
use uuid::Uuid;

use crate::metrics::Metrics;

#[derive(Debug, Clone)]
pub struct DevIdentity {
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: orbit_domain::UserRole,
}

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub bus: Arc<dyn EventBus>,
    pub registry: Arc<HarnessRegistry>,
    pub auth: Arc<AuthConfig>,
    pub jwt: Option<Arc<JwtValidator>>,
    pub default_org_id: Uuid,
    pub dev_identity: Option<Arc<DevIdentity>>,
    pub environment: String,
    pub events_driver: String,
    pub public_base_url: String,
    pub metrics: Arc<Metrics>,
    pub max_body_bytes: usize,
    pub cors_origins: Vec<String>,
    pub launcher_url: Option<String>,
    pub launcher_token: Option<String>,
    pub worker_api_url: String,
    /// Archive terminal runs older than this many days. `None` disables retention.
    pub archive_after_days: Option<u32>,
}

impl AppState {
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    pub fn is_dev_mode(&self) -> bool {
        matches!(self.auth.mode, orbit_security::AuthMode::Dev)
    }

    pub fn auth_cors_origins(&self) -> &[String] {
        &self.cors_origins
    }
}
