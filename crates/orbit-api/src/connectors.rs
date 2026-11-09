//! Built-in connector manifests.
//!
//! Phase 0 ships manifests and connection management. Capability invocation, OAuth
//! flows and webhook verification are implemented in Phase 2; manifests are already the
//! contract the platform and UI consume.

use orbit_contracts::{AuthMethod, CapabilityDescriptor, ConnectorManifest};
use orbit_domain::CapabilityEffect;

fn capability(
    id: &str,
    name: &str,
    description: &str,
    effect: CapabilityEffect,
) -> CapabilityDescriptor {
    CapabilityDescriptor {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        effect,
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: serde_json::json!({ "type": "object" }),
    }
}

pub fn builtin_manifests() -> Vec<ConnectorManifest> {
    vec![
        ConnectorManifest {
            slug: "mock".into(),
            name: "Mock connector".into(),
            version: "0.1.0".into(),
            description: "Deterministic connector for local development and tests.".into(),
            auth: vec![AuthMethod::None],
            webhooks: false,
            capabilities: vec![
                capability(
                    "notes.list",
                    "List notes",
                    "Returns demo notes.",
                    CapabilityEffect::Read,
                ),
                capability(
                    "notes.create",
                    "Create note",
                    "Creates a demo note.",
                    CapabilityEffect::Write,
                ),
            ],
            config_schema: serde_json::json!({ "type": "object", "properties": {} }),
            docs_url: None,
            builtin: true,
        },
        ConnectorManifest {
            slug: "telegram".into(),
            name: "Telegram".into(),
            version: "0.1.0".into(),
            description: "Telegram bot messaging (Phase 2 invocation).".into(),
            auth: vec![AuthMethod::ApiToken],
            webhooks: true,
            capabilities: vec![
                capability(
                    "messages.list",
                    "List updates",
                    "Reads recent bot updates.",
                    CapabilityEffect::Read,
                ),
                capability(
                    "messages.send",
                    "Send message",
                    "Sends a Telegram message.",
                    CapabilityEffect::ExternalSideEffect,
                ),
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "properties": { "chatId": { "type": "string" } }
            }),
            docs_url: Some("https://core.telegram.org/bots/api".into()),
            builtin: true,
        },
        ConnectorManifest {
            slug: "gitlab".into(),
            name: "GitLab".into(),
            version: "0.1.0".into(),
            description: "GitLab projects, issues and pipelines (Phase 2 invocation).".into(),
            auth: vec![AuthMethod::OAuth2, AuthMethod::ApiToken],
            webhooks: true,
            capabilities: vec![
                capability(
                    "projects.list",
                    "List projects",
                    "Lists accessible projects.",
                    CapabilityEffect::Read,
                ),
                capability(
                    "issues.list",
                    "List issues",
                    "Lists project issues.",
                    CapabilityEffect::Read,
                ),
                capability(
                    "issues.get",
                    "Get issue",
                    "Retrieves one issue.",
                    CapabilityEffect::Read,
                ),
                capability(
                    "issues.create",
                    "Create issue",
                    "Creates an issue.",
                    CapabilityEffect::Write,
                ),
                capability(
                    "issues.update",
                    "Update issue",
                    "Updates issue metadata.",
                    CapabilityEffect::Write,
                ),
                capability(
                    "merge_requests.create",
                    "Create merge request",
                    "Opens a merge request.",
                    CapabilityEffect::ExternalSideEffect,
                ),
                capability(
                    "pipelines.status",
                    "Pipeline status",
                    "Reads pipeline status.",
                    CapabilityEffect::Read,
                ),
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "properties": { "baseUrl": { "type": "string" } }
            }),
            docs_url: Some("https://docs.gitlab.com/ee/api/".into()),
            builtin: true,
        },
        ConnectorManifest {
            slug: "x".into(),
            name: "X".into(),
            version: "0.1.0".into(),
            description: "X/Twitter monitoring and posting (Phase 2).".into(),
            auth: vec![AuthMethod::OAuth2, AuthMethod::ApiToken],
            webhooks: false,
            capabilities: vec![
                capability(
                    "posts.search",
                    "Search posts",
                    "Searches recent posts.",
                    CapabilityEffect::Read,
                ),
                capability(
                    "posts.create",
                    "Post",
                    "Publishes a post.",
                    CapabilityEffect::ExternalSideEffect,
                ),
            ],
            config_schema: serde_json::json!({ "type": "object" }),
            docs_url: Some("https://developer.x.com/en/docs".into()),
            builtin: true,
        },
        ConnectorManifest {
            slug: "webhook".into(),
            name: "Generic webhook".into(),
            version: "0.1.0".into(),
            description: "Send and receive signed JSON webhooks.".into(),
            auth: vec![AuthMethod::HmacWebhook, AuthMethod::None],
            webhooks: true,
            capabilities: vec![capability(
                "request.send",
                "Send request",
                "Sends an outbound HTTP request.",
                CapabilityEffect::ExternalSideEffect,
            )],
            config_schema: serde_json::json!({
                "type": "object",
                "properties": { "url": { "type": "string" } }
            }),
            docs_url: None,
            builtin: true,
        },
    ]
}
