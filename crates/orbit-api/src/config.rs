use std::time::Duration;

use orbit_security::AuthConfig;

#[derive(Clone, Debug)]
pub struct ApiConfig {
    pub environment: String,
    pub http_addr: String,
    pub database_url: String,
    pub db_max_connections: u32,
    pub auto_migrate: bool,
    pub cors_origins: Vec<String>,
    pub seed_demo: bool,
    /// Seed a launchable mock worker profile when a launcher is configured.
    pub seed_worker_profile: bool,
    /// Tag used for generated worker-profile image defaults.
    pub image_tag: String,
    pub default_org_slug: String,
    pub default_org_name: String,
    pub public_base_url: String,
    pub body_limit_bytes: usize,
    pub auth: AuthConfig,
    pub master_key: String,
    pub launcher_url: Option<String>,
    pub launcher_token: Option<String>,
    pub worker_api_url: String,
    /// Days after which terminal executions, team runs and worker launches are archived.
    /// `0` disables the retention loop.
    pub archive_after_days: Option<u32>,
}

impl ApiConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let environment = env_or("ORBIT_ENV", "development");
        let auth = AuthConfig::from_env()?;
        let master_key = resolve_master_key(&environment)?;
        let cors_origins = env_or("ORBIT_CORS_ORIGINS", "http://localhost:3000")
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();

        Ok(Self {
            environment,
            http_addr: env_or("ORBIT_HTTP_ADDR", "0.0.0.0:8080"),
            database_url: env_or(
                "ORBIT_DATABASE_URL",
                "postgres://orbit:orbit@localhost:5432/orbit",
            ),
            db_max_connections: env_or("ORBIT_DB_MAX_CONNECTIONS", "10")
                .parse()
                .unwrap_or(10),
            auto_migrate: env_bool("ORBIT_DB_AUTO_MIGRATE", true),
            cors_origins,
            seed_demo: env_bool("ORBIT_SEED_DEMO", true),
            seed_worker_profile: env_bool("ORBIT_SEED_WORKER_PROFILE", true),
            image_tag: env_or("ORBIT_IMAGE_TAG", "latest"),
            default_org_slug: env_or("ORBIT_DEFAULT_ORG_SLUG", "orbit"),
            default_org_name: env_or("ORBIT_DEFAULT_ORG_NAME", "ORBIT"),
            public_base_url: env_or("ORBIT_PUBLIC_BASE_URL", "http://localhost:8080"),
            body_limit_bytes: env_or("ORBIT_BODY_LIMIT_BYTES", "2097152")
                .parse()
                .unwrap_or(2 * 1024 * 1024),
            auth,
            master_key,
            launcher_url: std::env::var("ORBIT_LAUNCHER_URL")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            launcher_token: std::env::var("ORBIT_LAUNCHER_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            worker_api_url: env_or("ORBIT_WORKER_API_URL", "http://orbit-api:8080"),
            archive_after_days: env_or("ORBIT_ARCHIVE_AFTER_DAYS", "30")
                .parse::<u32>()
                .ok()
                .filter(|days| *days > 0),
        })
    }

    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
}

fn resolve_master_key(environment: &str) -> anyhow::Result<String> {
    if let Ok(key) = std::env::var("ORBIT_MASTER_KEY") {
        if !key.trim().is_empty() {
            return Ok(key);
        }
    }
    if environment == "production" {
        anyhow::bail!(
            "ORBIT_MASTER_KEY is required in production (base64-encoded 32-byte key); \
             generate one with `openssl rand -base64 32`"
        );
    }
    // Deterministic, clearly non-production development key so local secrets survive
    // restarts. Never used when ORBIT_ENV=production.
    use base64::Engine;
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"orbit-development-master-key-do-not-use-in-production");
    tracing::warn!(
        "ORBIT_MASTER_KEY is not set; using an insecure development key. Set ORBIT_MASTER_KEY."
    );
    Ok(base64::engine::general_purpose::STANDARD.encode(hasher.finalize()))
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
        .unwrap_or(default)
}
