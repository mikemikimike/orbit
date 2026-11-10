use std::sync::Arc;

use orbit_domain::RuntimeKind;
use orbit_runtime::registry::RuntimeRegistry;
use orbit_runtime::testing::{assert_adapter_contract, assert_fails_closed};

#[tokio::test]
async fn mock_adapter_satisfies_contract() {
    let registry = RuntimeRegistry::builtin();
    let adapter = registry.require(RuntimeKind::Mock).unwrap();
    assert_adapter_contract(adapter).await;
}

#[tokio::test]
async fn unimplemented_adapters_fail_closed() {
    let registry = RuntimeRegistry::builtin();
    for kind in [
        RuntimeKind::ClaudeCode,
        RuntimeKind::Pi,
        RuntimeKind::Generic,
    ] {
        let adapter = registry.require(kind).unwrap();
        assert_fails_closed(adapter).await;
    }
}

#[tokio::test]
async fn registry_exposes_all_runtime_families() {
    let registry = RuntimeRegistry::builtin();
    let kinds = registry.kinds();
    for kind in RuntimeKind::ALL {
        assert!(kinds.contains(&kind), "missing runtime {kind}");
    }
    let descriptors = registry.descriptors();
    assert_eq!(descriptors.len(), RuntimeKind::ALL.len());
    let opencode = descriptors
        .iter()
        .find(|d| d.kind == RuntimeKind::OpenCode)
        .expect("opencode descriptor");
    assert!(opencode.capabilities.runtime_managed_providers);
    assert_eq!(
        opencode.implementation_status.as_str(),
        "available",
        "OpenCode execution is covered by tests/opencode_e2e.rs against v2.2.5"
    );
}

#[tokio::test]
async fn opencode_preflight_reports_unavailable_binary() {
    let registry = RuntimeRegistry::builtin();
    let adapter: Arc<dyn orbit_runtime::RuntimeAdapter> =
        registry.require(RuntimeKind::OpenCode).unwrap();
    let health = adapter
        .health(&orbit_contracts::RuntimeConfig {
            kind: RuntimeKind::OpenCode,
            settings: serde_json::json!({ "binary": "definitely-not-installed-opencode" }),
        })
        .await
        .expect("health returns degraded rather than failing");
    assert!(!health.healthy);
}
