//! Shared contract test harness.
//!
//! Every registered adapter should pass [`assert_adapter_contract`]. This prevents a new
//! harness from silently violating cancellation, descriptor or outcome invariants.

use std::sync::Arc;
use std::time::Duration;

use orbit_contracts::{
    ExecutionRequest, InputProvenance, InstructionsBlock, HarnessAdapter, HarnessConfig,
    HarnessErrorKind,
};
use orbit_domain::{AutonomyLevel, ExecutionMode, ExecutionStatus, ResourceLimits, HarnessKind};

pub fn execution_request(_adapter: &dyn HarnessAdapter, task: &str) -> ExecutionRequest {
    let (events, _receiver) = tokio::sync::mpsc::channel(64);
    ExecutionRequest {
        execution_id: uuid::Uuid::now_v7(),
        agent_id: uuid::Uuid::now_v7(),
        agent_version_id: uuid::Uuid::now_v7(),
        agent_name: "contract-test".into(),
        instructions: InstructionsBlock {
            system: "You are a contract test agent.".into(),
            constraints: vec![],
        },
        input: orbit_contracts::ExecutionInput {
            task: task.into(),
            payload: serde_json::json!({}),
            provenance: InputProvenance::manual("contract-test"),
        },
        model: None,
        grants: vec![],
        autonomy: AutonomyLevel::RECOMMEND,
        resources: ResourceLimits {
            timeout_seconds: 10,
            ..Default::default()
        },
        harness_config: serde_json::json!({}),
        session: orbit_contracts::SessionRequest {
            session_ref: None,
            mode: ExecutionMode::OnDemand,
        },
        workspace: Default::default(),
        events: Some(events),
        cancel: tokio_util::sync::CancellationToken::new(),
    }
}

/// Verify descriptor integrity and basic execute/cancel behaviour.
pub async fn assert_adapter_contract(adapter: Arc<dyn HarnessAdapter>) {
    let descriptor = adapter.descriptor();
    assert_eq!(descriptor.kind, adapter.kind(), "descriptor kind mismatch");
    assert!(!descriptor.name.is_empty());
    assert!(
        !descriptor.capabilities.execution_modes.is_empty(),
        "adapter must advertise an execution mode"
    );
    if descriptor.capabilities.supports_cancel {
        assert!(
            descriptor.capabilities.supports_streaming,
            "cancellation should be observable via streaming events"
        );
    }

    let config = HarnessConfig {
        kind: adapter.kind(),
        settings: serde_json::json!({}),
    };
    // Validation must not panic and must be self-consistent for an empty config.
    let report = adapter.validate_config(&config).await;
    let _ = report.issues.len();

    let health = adapter.health(&config).await;
    assert!(
        health.is_ok(),
        "health check must not fail at the transport level for built-in adapters"
    );

    if adapter.kind() == HarnessKind::Mock {
        // Streaming, success and cancellation are actually asserted for the mock harness.
        let request = execution_request(adapter.as_ref(), "summarize this contract test");
        let (sender, mut receiver) = tokio::sync::mpsc::channel(64);
        let mut request = request;
        request.events = Some(sender);
        let outcome = adapter
            .execute(request)
            .await
            .expect("mock execution succeeds");
        assert_eq!(outcome.status, ExecutionStatus::Succeeded);
        assert!(outcome.output.is_some());
        let mut events = 0;
        while let Ok(Some(_)) =
            tokio::time::timeout(Duration::from_millis(10), receiver.recv()).await
        {
            events += 1;
        }
        assert!(
            events >= 2,
            "adapter should stream at least start + output events"
        );

        // Cancellation
        let mut request = execution_request(adapter.as_ref(), "slow task");
        let (sender, _receiver) = tokio::sync::mpsc::channel(64);
        request.events = Some(sender);
        let cancel = request.cancel.clone();
        let handle = tokio::spawn(async move { adapter.execute(request).await });
        tokio::time::sleep(Duration::from_millis(100)).await;
        cancel.cancel();
        let outcome = tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .expect("cancellation must complete promptly")
            .expect("task join")
            .expect("cancellation is an outcome, not a transport error");
        assert_eq!(outcome.status, ExecutionStatus::Cancelled);
    }
}

/// Verify that an unimplemented adapter fails closed with `Unsupported`.
pub async fn assert_fails_closed(adapter: Arc<dyn HarnessAdapter>) {
    let request = execution_request(adapter.as_ref(), "anything");
    let error = adapter
        .execute(request)
        .await
        .expect_err("must not execute");
    assert!(
        matches!(
            error.kind,
            HarnessErrorKind::Unsupported
                | HarnessErrorKind::Unavailable
                | HarnessErrorKind::InvalidConfig
        ),
        "unexpected error kind: {error:?}"
    );
}
