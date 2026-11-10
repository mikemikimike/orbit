use orbit_contracts::{
    ModelEndpoint, ModelSelection, ProviderCompat, ProviderKind, ResolvedModel, SecretLease,
};
use orbit_domain::{AgentSpec, ModelManagement};
use orbit_store::Store;

/// Resolve the agent's model selection into what a harness adapter needs.
///
/// Credentials are decrypted only here, held in memory for the execution and never
/// serialized or logged. A missing credential yields a clear, non-retryable error.
pub async fn resolve_model(
    store: &Store,
    org_id: uuid::Uuid,
    spec: &AgentSpec,
) -> Result<Option<ResolvedModel>, String> {
    let provider_name = spec.model.provider.clone();
    let model = spec.model.model.clone();

    if spec.model.management == ModelManagement::Harness {
        let mut resolved = ResolvedModel::harness_managed(provider_name.clone(), model);
        if let Some(name) = &provider_name {
            if let Some(provider) = store
                .get_provider_by_name(org_id, name)
                .await
                .map_err(|e| e.to_string())?
            {
                if let Some(reference) = &provider.secret_ref {
                    let value = store
                        .get_secret_value(reference)
                        .await
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| {
                            format!(
                                "credential for provider '{name}' is missing from the secret store"
                            )
                        })?;
                    resolved.credential = Some(SecretLease::new(reference, value.expose()));
                }
            }
        }
        return Ok(Some(resolved));
    }

    let name = provider_name.ok_or_else(|| {
        "model management is 'platform' but no provider is configured".to_string()
    })?;
    let provider = store
        .get_provider_by_name(org_id, &name)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("provider '{name}' does not exist"))?;
    let kind = ProviderKind::parse(&provider.kind)
        .ok_or_else(|| format!("provider '{name}' has an unknown kind '{}'", provider.kind))?;
    if kind != ProviderKind::OpenAiCompatible {
        return Err(format!(
            "provider '{name}' is harness-managed and cannot be used with platform model management"
        ));
    }
    if !provider.enabled {
        return Err(format!("provider '{name}' is disabled"));
    }

    let base_url = provider
        .config
        .get("baseUrl")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("provider '{name}' is missing baseUrl"))?
        .to_string();
    let model_id = model
        .or_else(|| {
            provider
                .config
                .get("model")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .ok_or_else(|| format!("provider '{name}' is missing a model id"))?;
    let mut headers = std::collections::BTreeMap::new();
    if let Some(map) = provider.config.get("headers").and_then(|v| v.as_object()) {
        for (key, value) in map {
            if let Some(value) = value.as_str() {
                headers.insert(key.clone(), value.to_string());
            }
        }
    }
    let compat: ProviderCompat = provider
        .config
        .get("compat")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();
    let request_params = provider
        .config
        .get("requestParams")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let context_window = provider
        .config
        .get("contextWindow")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let max_output_tokens = provider
        .config
        .get("maxOutputTokens")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    let credential = match &provider.secret_ref {
        Some(reference) => {
            let value = store
                .get_secret_value(reference)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| {
                    format!("credential for provider '{name}' is missing from the secret store")
                })?;
            Some(SecretLease::new(reference, value.expose()))
        }
        None => None,
    };

    Ok(Some(ResolvedModel {
        selection: ModelSelection::Platform {
            provider_id: provider.id,
            provider_name: provider.name,
            model: model_id.clone(),
        },
        endpoint: Some(ModelEndpoint {
            base_url,
            model: model_id,
            headers,
            request_params,
            compat,
            context_window,
            max_output_tokens,
        }),
        credential,
    }))
}
