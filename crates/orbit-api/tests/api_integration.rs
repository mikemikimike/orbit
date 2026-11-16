use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::Engine;
use http_body_util::BodyExt;
use orbit_api::{build_router, AppState, DevIdentity};
use orbit_domain::UserRole;
use orbit_events::MemoryEventBus;
use orbit_harness::HarnessRegistry;
use orbit_security::{AuthConfig, AuthMode, DevAuthConfig, SecretCipher};
use orbit_store::Store;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

struct TestApp {
    router: axum::Router,
    store: Store,
    org_id: Uuid,
    agent_id: Uuid,
}

async fn test_app(allow_anonymous: bool) -> Option<TestApp> {
    let url = std::env::var("ORBIT_TEST_DATABASE_URL").ok()?;
    let key = base64::engine::general_purpose::STANDARD.encode([9u8; 32]);
    let cipher = Arc::new(SecretCipher::from_base64_key(&key).unwrap());
    let store = Store::connect(&url, 5)
        .await
        .expect("connect")
        .with_cipher(cipher);
    store.migrate().await.expect("migrate");

    let slug = format!("api-test-{}", Uuid::now_v7());
    let org = store
        .create_organization(&slug, "API Test", orbit_domain::OrganizationMode::Personal)
        .await
        .unwrap();
    let user = store
        .ensure_user(org.id, "api@test.local", "API Test", UserRole::Owner)
        .await
        .unwrap();

    // Seed one connector so connection endpoints work.
    for manifest in orbit_api::connectors::builtin_manifests() {
        store.upsert_connector_manifest(&manifest).await.unwrap();
    }
    store
        .upsert_harness_descriptor(
            &HarnessRegistry::builtin()
                .descriptors()
                .into_iter()
                .find(|d| d.kind == orbit_domain::HarnessKind::Mock)
                .unwrap(),
        )
        .await
        .unwrap();

    let auth_config = AuthConfig {
        mode: AuthMode::Dev,
        dev: DevAuthConfig {
            email: user.email.clone(),
            display_name: user.display_name.clone(),
            org_slug: org.slug.clone(),
            org_name: org.name.clone(),
            role: UserRole::Owner,
            allow_anonymous: true,
            dev_token: "test-token".into(),
        },
        oidc: None,
        allow_api_tokens: true,
    };

    let bus = Arc::new(MemoryEventBus::new(128));
    let state = AppState {
        store: store.clone(),
        bus,
        registry: Arc::new(HarnessRegistry::builtin()),
        auth: Arc::new(auth_config),
        jwt: None,
        default_org_id: org.id,
        dev_identity: Some(Arc::new(DevIdentity {
            user_id: user.id,
            org_id: org.id,
            email: user.email.clone(),
            display_name: user.display_name.clone(),
            role: UserRole::Owner,
        })),
        environment: "test".into(),
        events_driver: "memory".into(),
        public_base_url: "http://localhost".into(),
        metrics: Arc::new(orbit_api::metrics::Metrics::new()),
        max_body_bytes: 2 * 1024 * 1024,
        cors_origins: vec!["*".into()],
        launcher_url: None,
        launcher_token: None,
        worker_api_url: "http://localhost:8080".into(),
        archive_after_days: None,
        license: orbit_security::LicenseHandle::load(orbit_security::LicenseSource {
            token: None,
            file: None,
        }),
    };
    // In this test harness anonymous is controlled by auth_config.dev.allow_anonymous;
    // rebuild the config accordingly for the negative auth test.
    let mut state = state;
    let mut auth = (*state.auth).clone();
    auth.dev.allow_anonymous = allow_anonymous;
    state.auth = Arc::new(auth);

    let created = create_demo_agent(&store, org.id, user.id).await;
    Some(TestApp {
        router: build_router(state),
        store,
        org_id: org.id,
        agent_id: created,
    })
}

fn definition() -> Value {
    json!({
        "apiVersion": "orbit.io/v1",
        "kind": "Agent",
        "metadata": { "name": "API Test Agent", "description": "created by tests" },
        "spec": {
            "harness": { "type": "mock", "executionMode": "on_demand", "config": {} },
            "model": { "management": "harness" },
            "instructions": { "system": "You are a test agent." },
            "connectors": [],
            "triggers": [{ "type": "schedule", "cron": "0 * * * *" }],
            "autonomy": { "level": 2 },
            "resources": { "timeoutSeconds": 30, "maxConcurrentExecutions": 1 }
        }
    })
}

async fn create_demo_agent(store: &Store, org_id: Uuid, user_id: Uuid) -> Uuid {
    let definition: orbit_domain::AgentDefinition = serde_json::from_value(definition()).unwrap();
    let (agent, _) = store
        .create_agent(org_id, Some(user_id), &definition)
        .await
        .unwrap();
    store
        .set_agent_status(org_id, agent.id, orbit_domain::AgentStatus::Active)
        .await
        .unwrap();
    agent.id
}

async fn send(router: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = router.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, json)
}

fn get(path: &str) -> Request<Body> {
    Request::builder().uri(path).body(Body::empty()).unwrap()
}

fn post(path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn public_endpoints_and_meta() {
    let Some(app) = test_app(true).await else {
        return;
    };
    let (status, body) = send(&app.router, get("/healthz")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");

    let (status, body) = send(&app.router, get("/api/v1/meta")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["apiVersion"], "v1");
    assert_eq!(body["auth"]["mode"], "dev");

    let (status, body) = send(&app.router, get("/api/v1/openapi.json")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["paths"]["/api/v1/agents"].is_object());
}

#[tokio::test]
async fn unauthenticated_requests_are_rejected_when_anonymous_disabled() {
    let Some(app) = test_app(false).await else {
        return;
    };
    let (status, body) = send(&app.router, get("/api/v1/agents")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "unauthorized");
    assert!(!body["error"]["requestId"].as_str().unwrap_or("").is_empty());

    let authed = Request::builder()
        .uri("/api/v1/agents")
        .header("authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();
    let (status, _) = send(&app.router, authed).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn agent_crud_validation_and_export() {
    let Some(app) = test_app(true).await else {
        return;
    };

    let (status, body) = send(&app.router, get("/api/v1/agents?limit=10")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["harness"], "mock");

    let (status, body) = send(&app.router, post("/api/v1/agents", definition())).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["telegram_test"], Value::Null);
    assert_eq!(body["spec"]["autonomy"]["level"], 2);

    // Unknown harness → validation error envelope.
    let mut invalid = definition();
    invalid["spec"]["harness"]["type"] = json!("not-a-harness");
    let (status, body) = send(&app.router, post("/api/v1/agents", invalid)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "validation_error");

    // Semantic validation (bad cron) → 422 with field details.
    let mut invalid = definition();
    invalid["spec"]["triggers"] = json!([{ "type": "schedule", "cron": "nope" }]);
    invalid["metadata"]["name"] = json!("Invalid Agent");
    let (status, body) = send(&app.router, post("/api/v1/agents", invalid)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "validation_error");
    assert!(body["error"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["path"].as_str().unwrap_or("").contains("triggers")));

    // Export
    let (status, _) = send(&app.router, get(&format!("/api/v1/agents/{id}/export"))).await;
    assert_eq!(status, StatusCode::OK);

    // 404 envelope
    let (status, body) = send(
        &app.router,
        get(&format!("/api/v1/agents/{}", Uuid::now_v7())),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "not_found");
}

#[tokio::test]
async fn execution_creation_is_idempotent_and_streams_events() {
    let Some(app) = test_app(true).await else {
        return;
    };
    let path = format!("/api/v1/agents/{}/executions", app.agent_id);
    let request = |key: &str| {
        Request::builder()
            .method("POST")
            .uri(&path)
            .header("content-type", "application/json")
            .header("idempotency-key", key)
            .body(Body::from(json!({ "task": "test task" }).to_string()))
            .unwrap()
    };
    let (status, body) = send(&app.router, request("api-test-idem")).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    let execution_id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["status"], "queued");

    let (status, body2) = send(&app.router, request("api-test-idem")).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body2["id"], body["id"]);

    // Simulate the worker's durable writes so we can verify read endpoints.
    let execution_id = Uuid::parse_str(&execution_id).unwrap();
    app.store
        .claim_execution(execution_id, "test-worker")
        .await
        .unwrap();
    app.store
        .mark_execution_running(execution_id, Some("session-test"))
        .await
        .unwrap();
    app.store
        .append_execution_event(
            app.org_id,
            execution_id,
            "log",
            orbit_domain::LogLevel::Info,
            "hello from test",
            &json!({}),
        )
        .await
        .unwrap();
    app.store
        .finish_execution(
            execution_id,
            orbit_domain::ExecutionStatus::Succeeded,
            Some(&json!({ "ok": true })),
            None,
            None,
            Some("session-test"),
        )
        .await
        .unwrap();

    let (status, body) = send(
        &app.router,
        get(&format!("/api/v1/executions/{execution_id}")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "succeeded");
    assert_eq!(body["output"]["ok"], true);

    let (status, body) = send(
        &app.router,
        get(&format!("/api/v1/executions/{execution_id}/events")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["message"] == "hello from test"));

    // SSE replay includes snapshot, the stored event and a done marker.
    let response = app
        .router
        .clone()
        .oneshot(get(&format!(
            "/api/v1/executions/{execution_id}/events/stream"
        )))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("event: snapshot"), "{text}");
    assert!(text.contains("event: execution.event"), "{text}");
    assert!(text.contains("event: done"), "{text}");
}

#[tokio::test]
async fn approvals_can_be_listed_and_decided() {
    let Some(app) = test_app(true).await else {
        return;
    };
    let (status, execution) = send(
        &app.router,
        post(
            &format!("/api/v1/agents/{}/executions", app.agent_id),
            json!({ "task": "approval test" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{execution}");
    let execution_id = Uuid::parse_str(execution["id"].as_str().unwrap()).unwrap();
    let approval = app
        .store
        .create_approval(
            app.org_id,
            execution_id,
            json!({ "capability": "issues.comment", "effect": "write" }),
            None,
        )
        .await
        .unwrap();

    let (status, body) = send(&app.router, get("/api/v1/approvals?status=pending")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["id"] == approval.id.to_string()));

    let (status, body) = send(
        &app.router,
        post(
            &format!("/api/v1/approvals/{}/approve", approval.id),
            json!({ "comment": "looks good" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "approved");
    assert_eq!(body["comment"], "looks good");

    // Deciding twice is a conflict, not a silent overwrite.
    let (status, body) = send(
        &app.router,
        post(
            &format!("/api/v1/approvals/{}/approve", approval.id),
            json!({}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "conflict");
}

#[tokio::test]
async fn provider_credentials_are_write_only() {
    let Some(app) = test_app(true).await else {
        return;
    };
    let body = json!({
        "name": "Test Gateway",
        "kind": "openai_compatible",
        "config": { "baseUrl": "http://localhost:9999/v1", "model": "qwen" },
        "credential": "sk-test-should-never-leak"
    });
    let (status, body) = send(&app.router, post("/api/v1/providers", body)).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["hasCredential"], true);
    assert!(!body.to_string().contains("sk-test-should-never-leak"));
    let provider_id = body["id"].as_str().unwrap();

    let (status, body) = send(&app.router, get("/api/v1/providers")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body.to_string().contains("sk-test-should-never-leak"));
    assert_eq!(body[0]["hasCredential"], true);

    // Secrets are stored encrypted.
    let ciphertext: String =
        sqlx::query_scalar("SELECT ciphertext FROM secrets WHERE reference LIKE 'provider/%'")
            .fetch_one(app.store.pool())
            .await
            .unwrap();
    assert!(!ciphertext.contains("sk-test-should-never-leak"));

    // Invalid provider config is rejected before persistence.
    let (status, body) = send(
        &app.router,
        post(
            "/api/v1/providers",
            json!({ "name": "Bad", "kind": "openai_compatible", "config": { "baseUrl": "ftp://x", "model": "m" } }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "validation_error");

    let (status, _) = send(
        &app.router,
        get(&format!("/api/v1/providers/{provider_id}")),
    )
    .await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn connections_are_org_scoped_and_credentials_hidden() {
    let Some(app) = test_app(true).await else {
        return;
    };
    let body = json!({
        "connectorSlug": "telegram",
        "name": "Telegram Bot",
        "credential": "123456:bot-secret",
        "scopes": ["messages.list"]
    });
    let (status, body) = send(&app.router, post("/api/v1/connections", body)).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["hasCredential"], true);
    assert_eq!(body["connectorSlug"], "telegram");
    assert!(!body.to_string().contains("bot-secret"));
    let connection_id = body["id"].as_str().unwrap().to_string();

    let (status, body) = send(&app.router, get("/api/v1/connections")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);

    // Unknown connector → 404
    let (status, body) = send(
        &app.router,
        post(
            "/api/v1/connections",
            json!({ "connectorSlug": "nope", "name": "x" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "not_found");

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/connections/{connection_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}
