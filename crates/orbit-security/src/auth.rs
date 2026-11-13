use std::collections::BTreeSet;
use std::str::FromStr;
use std::time::{Duration, Instant};

use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use orbit_domain::{OrganizationId, UserId, UserRole};

use crate::{Permission, SecurityError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    /// Self-hosted development: a seeded org/user, optional anonymous access.
    Dev,
    /// OIDC bearer tokens validated against the provider's JWKS.
    Oidc,
}

impl AuthMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Oidc => "oidc",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DevAuthConfig {
    pub email: String,
    pub display_name: String,
    pub org_slug: String,
    pub org_name: String,
    pub role: UserRole,
    pub allow_anonymous: bool,
    /// Convenience bearer token accepted in dev mode (`orbit-dev-token` by default).
    pub dev_token: String,
}

impl Default for DevAuthConfig {
    fn default() -> Self {
        Self {
            email: "dev@orbit.local".into(),
            display_name: "ORBIT Developer".into(),
            org_slug: "orbit-dev".into(),
            org_name: "ORBIT Development".into(),
            role: UserRole::Owner,
            allow_anonymous: true,
            dev_token: "orbit-dev-token".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OidcConfig {
    pub issuer: String,
    pub audience: String,
    pub jwks_url: String,
    /// OAuth/OIDC client id used by the web app. Defaults to the audience.
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub auto_provision: bool,
    pub default_role: UserRole,
    pub clock_skew_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub dev: DevAuthConfig,
    pub oidc: Option<OidcConfig>,
    pub allow_api_tokens: bool,
}

impl AuthConfig {
    /// Build from environment. Fails closed: OIDC mode without issuer/JWKS is an error.
    pub fn from_env() -> Result<Self, SecurityError> {
        let mode = match std::env::var("ORBIT_AUTH_MODE")
            .unwrap_or_else(|_| "dev".into())
            .as_str()
        {
            "dev" => AuthMode::Dev,
            "oidc" => AuthMode::Oidc,
            other => {
                return Err(SecurityError::Config(format!(
                    "unsupported ORBIT_AUTH_MODE: {other}"
                )))
            }
        };

        let dev = DevAuthConfig {
            email: env_or("ORBIT_DEV_USER_EMAIL", "dev@orbit.local"),
            display_name: env_or("ORBIT_DEV_USER_NAME", "ORBIT Developer"),
            org_slug: env_or("ORBIT_DEV_ORG_SLUG", "orbit-dev"),
            org_name: env_or("ORBIT_DEV_ORG_NAME", "ORBIT Development"),
            role: UserRole::from_str(&env_or("ORBIT_DEV_USER_ROLE", "owner"))
                .map_err(SecurityError::Config)?,
            allow_anonymous: env_bool("ORBIT_DEV_AUTH_ALLOW_ANONYMOUS", true),
            dev_token: env_or("ORBIT_DEV_TOKEN", "orbit-dev-token"),
        };

        let oidc = match mode {
            AuthMode::Dev => None,
            AuthMode::Oidc => {
                let issuer = std::env::var("ORBIT_OIDC_ISSUER").map_err(|_| {
                    SecurityError::Config("ORBIT_OIDC_ISSUER is required in oidc mode".into())
                })?;
                let audience = std::env::var("ORBIT_OIDC_AUDIENCE").map_err(|_| {
                    SecurityError::Config("ORBIT_OIDC_AUDIENCE is required in oidc mode".into())
                })?;
                let jwks_url = std::env::var("ORBIT_OIDC_JWKS_URL").map_err(|_| {
                    SecurityError::Config("ORBIT_OIDC_JWKS_URL is required in oidc mode".into())
                })?;
                let client_id = env_or("ORBIT_OIDC_CLIENT_ID", &audience);
                let redirect_uri = env_or(
                    "ORBIT_OIDC_REDIRECT_URI",
                    "http://localhost:3000/auth/callback",
                );
                Some(OidcConfig {
                    issuer,
                    audience,
                    jwks_url,
                    client_id,
                    client_secret: std::env::var("ORBIT_OIDC_CLIENT_SECRET")
                        .ok()
                        .filter(|value| !value.trim().is_empty()),
                    redirect_uri,
                    scopes: env_or("ORBIT_OIDC_SCOPES", "openid profile email")
                        .split_whitespace()
                        .map(str::to_string)
                        .collect(),
                    auto_provision: env_bool("ORBIT_OIDC_AUTO_PROVISION", true),
                    default_role: UserRole::from_str(&env_or("ORBIT_OIDC_DEFAULT_ROLE", "viewer"))
                        .map_err(SecurityError::Config)?,
                    clock_skew_seconds: env_or("ORBIT_OIDC_CLOCK_SKEW_SECONDS", "60")
                        .parse()
                        .unwrap_or(60),
                })
            }
        };

        Ok(Self {
            mode,
            dev,
            oidc,
            allow_api_tokens: env_bool("ORBIT_ALLOW_API_TOKENS", true),
        })
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
        .unwrap_or(default)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    Dev,
    Oidc,
    ApiToken,
    Anonymous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthContext {
    pub user_id: UserId,
    pub org_id: OrganizationId,
    pub email: String,
    pub display_name: String,
    pub role: UserRole,
    pub method: AuthMethod,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_id: Option<Uuid>,
}

impl AuthContext {
    pub fn can(&self, permission: Permission) -> bool {
        if self.scopes.is_empty() {
            return Permission::allows(self.role, permission);
        }
        let scopes: BTreeSet<&str> = self.scopes.iter().map(String::as_str).collect();
        scopes.contains(permission.as_str()) || scopes.contains("*")
    }

    pub fn require(&self, permission: Permission) -> Result<(), SecurityError> {
        if self.can(permission) {
            Ok(())
        } else {
            Err(SecurityError::Forbidden(format!(
                "missing permission {}",
                permission.as_str()
            )))
        }
    }

    pub fn is_service(&self) -> bool {
        self.method == AuthMethod::ApiToken
    }
}

/// Claims extracted from a validated OIDC access token / ID token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcIdentity {
    pub subject: String,
    pub issuer: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawClaims {
    sub: String,
    iss: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    preferred_username: Option<String>,
}

enum KeySource {
    Jwks {
        url: String,
        client: reqwest::Client,
        cache: tokio::sync::Mutex<Option<(JwkSet, Instant)>>,
        ttl: Duration,
    },
    Hmac(Vec<u8>),
}

/// Validates OIDC JWTs. JWKS is cached and refreshed once on unknown `kid`.
pub struct JwtValidator {
    issuer: String,
    audience: String,
    clock_skew: u64,
    keys: KeySource,
}

const ALLOWED_ALGORITHMS: [Algorithm; 6] = [
    Algorithm::RS256,
    Algorithm::RS384,
    Algorithm::RS512,
    Algorithm::ES256,
    Algorithm::ES384,
    Algorithm::PS256,
];

impl JwtValidator {
    pub fn new(config: &OidcConfig) -> Result<Self, SecurityError> {
        let client = crate::http::client_with_custom_ca(Duration::from_secs(10));
        Ok(Self {
            issuer: config.issuer.clone(),
            audience: config.audience.clone(),
            clock_skew: config.clock_skew_seconds,
            keys: KeySource::Jwks {
                url: config.jwks_url.clone(),
                client,
                cache: tokio::sync::Mutex::new(None),
                ttl: Duration::from_secs(300),
            },
        })
    }

    /// Test/dev constructor using a shared HS256 secret. Never used for real OIDC.
    pub fn hmac_for_tests(issuer: &str, audience: &str, secret: &[u8]) -> Self {
        Self {
            issuer: issuer.to_string(),
            audience: audience.to_string(),
            clock_skew: 5,
            keys: KeySource::Hmac(secret.to_vec()),
        }
    }

    pub async fn validate(&self, token: &str) -> Result<OidcIdentity, SecurityError> {
        let header = decode_header(token).map_err(|_| SecurityError::Unauthorized)?;
        if !ALLOWED_ALGORITHMS.contains(&header.alg) && !self.is_hmac() {
            return Err(SecurityError::Unauthorized);
        }
        let key = self.resolve_key(&header).await?;

        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[self.audience.as_str()]);
        validation.leeway = self.clock_skew;
        validation.validate_exp = true;

        let data = decode::<RawClaims>(token, &key, &validation)
            .map_err(|_| SecurityError::Unauthorized)?;
        let claims = data.claims;
        Ok(OidcIdentity {
            subject: claims.sub,
            issuer: claims.iss,
            email: claims.email,
            name: claims.name.or(claims.preferred_username),
        })
    }

    fn is_hmac(&self) -> bool {
        matches!(self.keys, KeySource::Hmac(_))
    }

    async fn resolve_key(
        &self,
        header: &jsonwebtoken::Header,
    ) -> Result<DecodingKey, SecurityError> {
        match &self.keys {
            KeySource::Hmac(secret) => Ok(DecodingKey::from_secret(secret)),
            KeySource::Jwks {
                url,
                client,
                cache,
                ttl,
            } => {
                let kid = header.kid.clone().ok_or(SecurityError::Unauthorized)?;
                {
                    let guard = cache.lock().await;
                    if let Some((set, fetched)) = guard.as_ref() {
                        if fetched.elapsed() < *ttl {
                            if let Some(jwk) = set.find(&kid) {
                                return DecodingKey::from_jwk(jwk)
                                    .map_err(|_| SecurityError::Unauthorized);
                            }
                        }
                    }
                }
                // Cache miss or expired: refresh once.
                let set: JwkSet = client
                    .get(url)
                    .send()
                    .await
                    .and_then(|r| r.error_for_status())
                    .map_err(|_| SecurityError::Unavailable("JWKS endpoint unreachable".into()))?
                    .json()
                    .await
                    .map_err(|_| SecurityError::Unauthorized)?;
                let key = set
                    .find(&kid)
                    .and_then(|jwk| DecodingKey::from_jwk(jwk).ok())
                    .ok_or(SecurityError::Unauthorized)?;
                *cache.lock().await = Some((set, Instant::now()));
                Ok(key)
            }
        }
    }
}

/// Hash an API token for storage/lookup. Tokens are high-entropy; SHA-256 is sufficient.
pub fn hash_api_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

/// Generate `(plaintext, hash)`. The plaintext is shown to the user exactly once.
pub fn generate_api_token() -> (String, String) {
    use base64::Engine;
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let token = format!(
        "orbit_sk_{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    );
    let hash = hash_api_token(&token);
    (token, hash)
}

/// One-time token material returned on creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuedApiToken {
    pub token: String,
    pub hash: String,
    pub prefix: String,
}

pub fn issue_api_token() -> IssuedApiToken {
    let (token, hash) = generate_api_token();
    let prefix = token.chars().take(16).collect();
    IssuedApiToken {
        token,
        hash,
        prefix,
    }
}

/// Worker tokens are separate from user API tokens so the API can reject them on
/// control-plane routes and accept them only on the worker surface.
pub fn issue_worker_token() -> IssuedApiToken {
    use base64::Engine;
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let token = format!(
        "orbit_wk_{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    );
    let hash = hash_api_token(&token);
    let prefix = token.chars().take(16).collect();
    IssuedApiToken {
        token,
        hash,
        prefix,
    }
}

/// Extract a bearer token from an Authorization header value.
pub fn bearer_token(header: Option<&str>) -> Option<&str> {
    let value = header?.trim();
    let (scheme, token) = value.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("bearer") && !token.trim().is_empty() {
        Some(token.trim())
    } else {
        None
    }
}

/// Used by the API to expose safe, non-secret metadata about the auth setup.
pub fn public_auth_metadata(config: &AuthConfig) -> Value {
    match &config.oidc {
        Some(oidc) => serde_json::json!({
            "mode": config.mode.as_str(),
            "issuer": oidc.issuer,
            "audience": oidc.audience,
            "scopes": oidc.scopes,
        }),
        None => serde_json::json!({
            "mode": config.mode.as_str(),
            "devUser": config.dev.email,
            "anonymousAllowed": config.dev.allow_anonymous,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_and_verifies_tokens() {
        let (token, hash) = generate_api_token();
        assert!(token.starts_with("orbit_sk_"));
        assert_eq!(hash, hash_api_token(&token));
        assert_ne!(hash, hash_api_token("other"));
    }

    #[test]
    fn parses_bearer_header() {
        assert_eq!(bearer_token(Some("Bearer abc")), Some("abc"));
        assert_eq!(bearer_token(Some("bearer abc")), Some("abc"));
        assert_eq!(bearer_token(Some("Basic abc")), None);
        assert_eq!(bearer_token(None), None);
    }
}
