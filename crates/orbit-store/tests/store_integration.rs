use std::sync::Arc;

use base64::Engine;
use orbit_domain::{
    AgentDefinition, AgentMetadata, AgentSpec, AgentStatus, AutonomySpec, ExecutionMode,
    ExecutionStatus, Instructions, MemorySpec, ModelSpec, ResourceLimits, RuntimeKind, RuntimeSpec,
    TriggerSource, TriggerSpec, UserRole,
};
use orbit_security::{SecretCipher, SecretValue};
use orbit_store::executions::NewExecution;
use orbit_store::Store;

async fn test_store() -> Option<Store> {
    let url = std::env::var("ORBIT_TEST_DATABASE_URL").ok()?;
    let key = base64::engine::general_purpose::STANDARD.encode([7u8; 32]);
    let cipher = Arc::new(SecretCipher::from_base64_key(&key).unwrap());
    let store = Store::connect(&url, 5)
        .await
        .expect("connect")
        .with_cipher(cipher);
    store.migrate().await.expect("migrate");
    Some(store)
}

async fn seed_org(store: &Store) -> (uuid::Uuid, uuid::Uuid) {
    let slug = format!("test-{}", uuid::Uuid::now_v7());
    let org = store
        .create_organization(&slug, "Test Org", orbit_domain::OrganizationMode::Personal)
        .await
        .expect("org");
    let user = store
        .ensure_user(org.id, "owner@test.local", "Test Owner", UserRole::Owner)
        .await
        .expect("user");
    (org.id, user.id)
}

fn definition(name: &str) -> AgentDefinition {
    AgentDefinition {
        api_version: orbit_domain::AGENT_API_VERSION.into(),
        kind: orbit_domain::AGENT_KIND.into(),
        metadata: AgentMetadata {
            name: name.into(),
            description: "integration test agent".into(),
            labels: Default::default(),
        },
        spec: AgentSpec {
            runtime: RuntimeSpec {
                kind: RuntimeKind::Mock,
                execution_mode: ExecutionMode::OnDemand,
                placement: orbit_domain::PlacementTarget::Auto,
                config: serde_json::json!({}),
            },
            model: ModelSpec::default(),
            instructions: Instructions {
                system: "You are a test agent.".into(),
                constraints: vec![],
            },
            connectors: vec![],
            skills: vec![],
            triggers: vec![TriggerSpec::Schedule {
                cron: "*/15 * * * *".into(),
                timezone: None,
            }],
            autonomy: AutonomySpec {
                level: 2,
                approval_mode: Default::default(),
                max_executions_per_hour: Some(10),
            },
            resources: ResourceLimits::default(),
            memory: MemorySpec::default(),
        },
    }
}

#[tokio::test]
async fn agent_lifecycle_and_versions() {
    let Some(store) = test_store().await else {
        eprintln!("skipping: ORBIT_TEST_DATABASE_URL not set");
        return;
    };
    let (org, user) = seed_org(&store).await;

    let (agent, version) = store
        .create_agent(org, Some(user), &definition("Lifecycle Agent"))
        .await
        .expect("create agent");
    assert_eq!(version.version, 1);
    assert_eq!(agent.current_version, 1);
    assert_eq!(agent.status, "draft");
    assert_eq!(agent.slug, "lifecycle-agent");

    // Schedule trigger was materialized from the spec.
    let triggers = store.list_triggers(org, Some(agent.id)).await.unwrap();
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].kind, "schedule");
    assert!(triggers[0].next_run_at.is_some());

    // Duplicate version is rejected.
    let err = store
        .add_agent_version(org, agent.id, Some(user), &definition("Lifecycle Agent"))
        .await
        .expect_err("duplicate spec must conflict");
    assert!(matches!(err, orbit_store::StoreError::Conflict(_)));

    // A changed spec creates version 2 and updates metadata.
    let mut changed = definition("Lifecycle Agent v2");
    changed.spec.autonomy.level = 3;
    let (agent, v2) = store
        .add_agent_version(org, agent.id, Some(user), &changed)
        .await
        .expect("new version");
    assert_eq!(v2.version, 2);
    assert_eq!(agent.current_version, 2);
    assert_eq!(agent.name, "Lifecycle Agent v2");

    let (rows, _, total) = store
        .list_agents(org, 10, None, None, Some("Lifecycle"))
        .await
        .unwrap();
    assert_eq!(total, 1);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].spec.is_some());

    // Lifecycle transitions
    let active = store
        .set_agent_status(org, agent.id, AgentStatus::Active)
        .await
        .unwrap();
    assert_eq!(active.status, "active");
    let paused = store
        .set_agent_status(org, agent.id, AgentStatus::Paused)
        .await
        .unwrap();
    assert_eq!(paused.status, "paused");
    assert!(store
        .set_agent_status(org, agent.id, AgentStatus::Paused)
        .await
        .is_err());
    assert_eq!(
        store
            .list_agent_versions(org, agent.id)
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn execution_lifecycle_and_events() {
    let Some(store) = test_store().await else {
        return;
    };
    let (org, user) = seed_org(&store).await;
    let (agent, version) = store
        .create_agent(org, Some(user), &definition("Execution Agent"))
        .await
        .unwrap();

    let input = serde_json::json!({
        "task": "hello",
        "payload": {},
        "provenance": { "source": "test", "trusted": true, "receivedAt": chrono::Utc::now() }
    });
    let execution = store
        .create_execution(NewExecution {
            org_id: org,
            agent_id: agent.id,
            agent_version_id: version.id,
            agent_name: &agent.name,
            runtime_kind: RuntimeKind::Mock,
            trigger_source: TriggerSource::Manual,
            placement: "auto",
            trigger_id: None,
            input: input.clone(),
            timeout_seconds: 30,
            requested_by: Some(user),
            idempotency_key: Some("idem-1"),
        })
        .await
        .unwrap();
    assert_eq!(execution.status, "queued");

    // Idempotent replay returns the same row.
    let replay = store
        .create_execution(NewExecution {
            org_id: org,
            agent_id: agent.id,
            agent_version_id: version.id,
            agent_name: &agent.name,
            runtime_kind: RuntimeKind::Mock,
            trigger_source: TriggerSource::Manual,
            placement: "auto",
            trigger_id: None,
            input,
            timeout_seconds: 30,
            requested_by: Some(user),
            idempotency_key: Some("idem-1"),
        })
        .await
        .unwrap();
    assert_eq!(replay.id, execution.id);

    // Competing claims: only one worker wins.
    let first = store
        .claim_execution(execution.id, "worker-a")
        .await
        .unwrap();
    let second = store
        .claim_execution(execution.id, "worker-b")
        .await
        .unwrap();
    assert!(first.is_some());
    assert!(second.is_none());

    store
        .mark_execution_running(execution.id, Some("session-1"))
        .await
        .unwrap();
    let event = store
        .append_execution_event(
            org,
            execution.id,
            "log",
            orbit_domain::LogLevel::Info,
            "working",
            &serde_json::json!({}),
        )
        .await
        .unwrap();
    assert_eq!(event.seq, 1);
    let second_event = store
        .append_execution_event(
            org,
            execution.id,
            "output",
            orbit_domain::LogLevel::Info,
            "done",
            &serde_json::json!({}),
        )
        .await
        .unwrap();
    assert_eq!(second_event.seq, 2);

    let finished = store
        .finish_execution(
            execution.id,
            ExecutionStatus::Succeeded,
            Some(&serde_json::json!({ "ok": true })),
            None,
            Some(&serde_json::json!({ "outputTokens": 3 })),
            Some("session-1"),
        )
        .await
        .unwrap()
        .expect("finished");
    assert_eq!(finished.status, "succeeded");
    assert!(finished.finished_at.is_some());

    // Terminal executions cannot be re-finished or cancelled.
    assert!(store
        .finish_execution(
            execution.id,
            ExecutionStatus::Failed,
            None,
            None,
            None,
            None
        )
        .await
        .unwrap()
        .is_none());
    assert!(store
        .request_execution_cancel(org, execution.id)
        .await
        .is_err());

    let events = store
        .list_execution_events(org, execution.id, 0, 100)
        .await
        .unwrap();
    assert_eq!(events.len(), 2);

    let summary = store.dashboard_summary(org).await.unwrap();
    assert_eq!(summary.total_agents, 1);
    assert!(summary.executions_24h >= 1);
}

#[tokio::test]
async fn secrets_are_encrypted_and_org_scoped_entities_work() {
    let Some(store) = test_store().await else {
        return;
    };
    let (org, user) = seed_org(&store).await;

    let reference = "provider/test-secret";
    store
        .put_secret(
            Some(org),
            reference,
            &SecretValue::new("sk-super-secret"),
            "test",
        )
        .await
        .unwrap();
    let value = store.get_secret_value(reference).await.unwrap().unwrap();
    assert_eq!(value.expose(), "sk-super-secret");

    // Raw ciphertext must not contain the plaintext.
    let ciphertext: String =
        sqlx::query_scalar("SELECT ciphertext FROM secrets WHERE reference = $1")
            .bind(reference)
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert!(!ciphertext.contains("sk-super-secret"));
    assert!(ciphertext.starts_with("v1:"));

    let provider = store
        .create_provider(
            org,
            "Gateway",
            "openai_compatible",
            &serde_json::json!({ "baseUrl": "http://localhost:8000/v1", "model": "qwen" }),
            Some(reference),
        )
        .await
        .unwrap();
    assert_eq!(provider.name, "Gateway");
    store
        .update_provider(org, provider.id, Some("Gateway 2"), None, None, Some(false))
        .await
        .unwrap();
    let providers = store.list_providers(org).await.unwrap();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].name, "Gateway 2");
    assert!(!providers[0].enabled);

    // Connections
    let manifest = orbit_contracts::ConnectorManifest {
        slug: format!("test-connector-{}", uuid::Uuid::now_v7()),
        name: "Test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        auth: vec![],
        webhooks: false,
        capabilities: vec![],
        config_schema: serde_json::json!({}),
        docs_url: None,
        builtin: false,
    };
    let connector = store.upsert_connector_manifest(&manifest).await.unwrap();
    let connection = store
        .create_connection(
            org,
            connector.id,
            "Primary",
            &serde_json::json!({}),
            &["notes.list".to_string()],
            Some(reference),
            Some(user),
        )
        .await
        .unwrap();
    let rows = store.list_connections(org).await.unwrap();
    assert!(rows
        .iter()
        .any(|row| row.connection.id == connection.id && row.connector_slug == manifest.slug));
    store.delete_connection(org, connection.id).await.unwrap();
    assert!(store.list_connections(org).await.unwrap().is_empty());
}

#[tokio::test]
async fn org_scoping_is_enforced() {
    let Some(store) = test_store().await else {
        return;
    };
    let (org_a, user_a) = seed_org(&store).await;
    let (org_b, _) = seed_org(&store).await;
    let (agent, _) = store
        .create_agent(org_a, Some(user_a), &definition("Scoped Agent"))
        .await
        .unwrap();

    assert!(store.get_agent(org_a, agent.id).await.is_ok());
    assert!(matches!(
        store.get_agent(org_b, agent.id).await,
        Err(orbit_store::StoreError::NotFound(_))
    ));
    assert!(!store
        .list_triggers(org_b, Some(agent.id))
        .await
        .unwrap()
        .iter()
        .any(|t| t.agent_id == agent.id));
}

#[tokio::test]
async fn runtime_catalog_upsert_is_idempotent() {
    let Some(store) = test_store().await else {
        return;
    };
    let (org, _) = seed_org(&store).await;
    let descriptor = orbit_runtime::RuntimeRegistry::builtin()
        .descriptors()
        .into_iter()
        .find(|d| d.kind == RuntimeKind::Mock)
        .unwrap();
    store.upsert_runtime_descriptor(&descriptor).await.unwrap();
    store.upsert_runtime_descriptor(&descriptor).await.unwrap();
    let runtimes = store.list_runtimes(org).await.unwrap();
    assert_eq!(
        runtimes.iter().filter(|r| r.kind == "mock").count(),
        1,
        "upsert must not duplicate built-in runtimes"
    );
}

#[tokio::test]
async fn orphaned_executions_are_failed_and_approvals_expired() {
    let Some(store) = test_store().await else {
        return;
    };
    let (org, user) = seed_org(&store).await;
    let (agent, version) = store
        .create_agent(org, Some(user), &definition("Reaper Agent"))
        .await
        .unwrap();

    let make_execution = |key: &'static str| {
        let store = &store;
        let agent = &agent;
        let version = &version;
        async move {
            store
                .create_execution(NewExecution {
                    org_id: org,
                    agent_id: agent.id,
                    agent_version_id: version.id,
                    agent_name: &agent.name,
                    runtime_kind: RuntimeKind::Mock,
                    trigger_source: TriggerSource::Manual,
                    placement: "auto",
                    trigger_id: None,
                    input: serde_json::json!({ "task": "x", "payload": {}, "provenance": { "source": "test", "trusted": true, "receivedAt": chrono::Utc::now() } }),
                    timeout_seconds: 30,
                    requested_by: Some(user),
                    idempotency_key: Some(key),
                })
                .await
                .unwrap()
        }
    };

    let orphan = make_execution("orphan-1").await;
    store
        .claim_execution(orphan.id, "worker-gone")
        .await
        .unwrap();
    store
        .mark_execution_running(orphan.id, Some("s1"))
        .await
        .unwrap();
    let approval = store
        .create_approval(
            org,
            orphan.id,
            serde_json::json!({ "capability": "issues.comment" }),
            None,
        )
        .await
        .unwrap();
    sqlx::query("UPDATE executions SET created_at = now() - interval '1 hour', started_at = now() - interval '1 hour' WHERE id = $1")
        .bind(orphan.id)
        .execute(store.pool())
        .await
        .unwrap();

    // A live worker's execution must not be reaped.
    let healthy = make_execution("healthy-1").await;
    store
        .claim_execution(healthy.id, "worker-alive")
        .await
        .unwrap();
    store
        .mark_execution_running(healthy.id, Some("s2"))
        .await
        .unwrap();
    store
        .upsert_worker_heartbeat("worker-alive", &["mock".into()], "local", Some("test"))
        .await
        .unwrap();

    let reaped = store
        .reap_orphaned_executions_for_org(org, 60)
        .await
        .unwrap();
    assert_eq!(reaped.len(), 1, "only the orphan should be reaped");
    assert_eq!(reaped[0].id, orphan.id);
    assert_eq!(reaped[0].status, "failed");
    assert_eq!(reaped[0].error.as_ref().unwrap()["code"], "worker_lost");

    assert_eq!(
        store.get_execution(org, healthy.id).await.unwrap().status,
        "running",
        "live worker execution must survive"
    );
    assert_eq!(
        store.get_approval(org, approval.id).await.unwrap().status,
        "expired"
    );
}

#[tokio::test]
async fn single_execution_mode_can_take_a_dispatched_execution() {
    let Some(store) = test_store().await else {
        return;
    };
    let (org, user) = seed_org(&store).await;
    let (agent, version) = store
        .create_agent(org, Some(user), &definition("Single Execution Agent"))
        .await
        .unwrap();
    let execution = store
        .create_execution(NewExecution {
            org_id: org,
            agent_id: agent.id,
            agent_version_id: version.id,
            agent_name: &agent.name,
            runtime_kind: RuntimeKind::Mock,
            trigger_source: TriggerSource::Manual,
            placement: "auto",
            trigger_id: None,
            input: serde_json::json!({ "task": "x", "payload": {}, "provenance": { "source": "test", "trusted": true, "receivedAt": chrono::Utc::now() } }),
            timeout_seconds: 30,
            requested_by: Some(user),
            idempotency_key: Some("single-1"),
        })
        .await
        .unwrap();
    store
        .claim_execution(execution.id, "parent-worker")
        .await
        .unwrap();
    let taken = store
        .take_execution_for_worker(execution.id, "container-worker")
        .await
        .unwrap()
        .expect("dispatched execution must be takeable");
    assert_eq!(taken.worker_id.as_deref(), Some("container-worker"));
}

#[tokio::test]
async fn skills_profiles_and_agent_scoped_claims() {
    let Some(store) = test_store().await else {
        return;
    };
    let (org, user) = seed_org(&store).await;

    let skill = store
        .create_skill(
            org,
            "issue-triage",
            "Issue triage",
            "Use when triaging issues",
            "---\nname: issue-triage\ndescription: triage\n---\n# Triage",
            &serde_json::json!({ "references/guide.md": "guide" }),
        )
        .await
        .unwrap();
    assert_eq!(skill.version, 1);
    let updated = store
        .update_skill(org, skill.id, Some("Triage v2"), None, None, None)
        .await
        .unwrap();
    assert_eq!(updated.version, 2);
    assert_eq!(
        store
            .skills_by_slugs(org, &["issue-triage".into()])
            .await
            .unwrap()
            .len(),
        1
    );

    let (agent_a, version_a) = store
        .create_agent(org, Some(user), &definition("Scoped Agent A"))
        .await
        .unwrap();
    let (agent_b, version_b) = store
        .create_agent(org, Some(user), &definition("Scoped Agent B"))
        .await
        .unwrap();

    let input = serde_json::json!({
        "task": "x", "payload": {},
        "provenance": { "source": "test", "trusted": true, "receivedAt": chrono::Utc::now() }
    });
    let exec_a = store
        .create_execution(NewExecution {
            org_id: org,
            agent_id: agent_a.id,
            agent_version_id: version_a.id,
            agent_name: &agent_a.name,
            runtime_kind: RuntimeKind::Mock,
            trigger_source: TriggerSource::Manual,
            placement: "auto",
            trigger_id: None,
            input: input.clone(),
            timeout_seconds: 30,
            requested_by: Some(user),
            idempotency_key: Some("scoped-a"),
        })
        .await
        .unwrap();
    let _exec_b = store
        .create_execution(NewExecution {
            org_id: org,
            agent_id: agent_b.id,
            agent_version_id: version_b.id,
            agent_name: &agent_b.name,
            runtime_kind: RuntimeKind::Mock,
            trigger_source: TriggerSource::Manual,
            placement: "auto",
            trigger_id: None,
            input,
            timeout_seconds: 30,
            requested_by: Some(user),
            idempotency_key: Some("scoped-b"),
        })
        .await
        .unwrap();

    let claimed = store
        .claim_next_queued("scoped-worker", &["mock".into()], "local", &[agent_a.id])
        .await
        .unwrap()
        .expect("agent A execution should be claimable");
    assert_eq!(claimed.id, exec_a.id);

    let profile = store
        .create_worker_profile(
            org,
            "Scoped Pi",
            "pi",
            "docker",
            "process",
            "orbit-worker-pi",
            "0.3.0",
            &[agent_a.id],
            &["issue-triage".into()],
            Some("opencode-go"),
            Some("deepseek-v4-pro"),
            Some("orbit-pi-auth"),
            &serde_json::json!({}),
            &serde_json::json!({}),
        )
        .await
        .unwrap();
    assert_eq!(profile.agent_ids, vec![agent_a.id]);
    assert_eq!(profile.skill_slugs, vec!["issue-triage"]);
    assert!(
        !store
            .set_worker_profile_enabled(org, profile.id, false)
            .await
            .unwrap()
            .enabled
    );
    store.delete_worker_profile(org, profile.id).await.unwrap();
}
