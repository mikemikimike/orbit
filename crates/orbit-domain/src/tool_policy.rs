//! Tool policy rules: per-call allow / deny / require-approval decisions.
//!
//! Rules can only make the platform decision stricter. A matching `deny` always wins,
//! then `require_approval`, then `allow`; an `allow` rule can never turn a platform
//! `require_approval` or `deny` into an execution. This keeps autonomy and capability
//! grants authoritative while allowing argument-level restrictions (paths, shell
//! commands, connections) on top.

use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::autonomy::{CapabilityEffect, PolicyDecision};

/// What a matching rule asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolPolicyAction {
    Allow,
    Deny,
    RequireApproval,
}

impl ToolPolicyAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::RequireApproval => "require_approval",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "allow" => Some(Self::Allow),
            "deny" => Some(Self::Deny),
            "require_approval" => Some(Self::RequireApproval),
            _ => None,
        }
    }
}

/// Fields a rule can match on. Every configured field must match (AND). An empty matcher
/// matches every tool call and is useful for a default `require_approval` policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct ToolPolicyMatcher {
    /// Glob against the tool name (`*` and `?` are wildcards).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// Glob against the connector capability, e.g. `issues.*`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<CapabilityEffect>,
    /// Glob against the connection name/slug.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    /// Glob against path-like arguments (`path`, `file_path`, `paths`, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_glob: Option<String>,
    /// Regular expression against shell commands (`command`, `cmd`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_regex: Option<String>,
}

impl ToolPolicyMatcher {
    pub fn is_empty(&self) -> bool {
        self.tool.is_none()
            && self.capability.is_none()
            && self.effect.is_none()
            && self.connection.is_none()
            && self.path_glob.is_none()
            && self.command_regex.is_none()
    }

    /// Compile the command regex, if configured. Validation uses this to reject bad rules
    /// before they are stored.
    pub fn compile(&self) -> Result<Option<Regex>, regex::Error> {
        self.command_regex
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(Regex::new)
            .transpose()
    }
}

/// A stored rule. `priority` orders evaluation for reporting only; the deny >
/// require_approval > allow precedence decides the outcome.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolPolicyRule {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub action: ToolPolicyAction,
    #[serde(default)]
    pub matcher: ToolPolicyMatcher,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Human-readable impact shown in the approval UI, e.g. "deletes files under /repo".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impact: Option<String>,
}

fn default_enabled() -> bool {
    true
}

/// The concrete tool call being evaluated.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub tool: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<CapabilityEffect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

/// Final decision plus provenance for audit/UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolPolicyOutcome {
    pub decision: PolicyDecision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact: Option<String>,
}

impl ToolPolicyOutcome {
    pub fn from_platform(decision: PolicyDecision) -> Self {
        Self {
            decision,
            rule_id: None,
            rule_name: None,
            reason: None,
            impact: None,
        }
    }

    pub fn is_allowed(&self) -> bool {
        self.decision == PolicyDecision::Allow
    }
}

/// Combine the platform decision with matching tool-policy rules.
///
/// `platform` is the hard floor and can never be weakened. `default_decision` applies
/// when no rule matches; it may be stricter than `platform` (for example native harness
/// tools default to `require_approval`) and an explicit `allow` rule may resolve it.
///
/// Precedence: platform `deny` wins, then any matching rule `deny`, then
/// `require_approval`, then `allow`. An `allow` rule never weakens `platform`.
pub fn evaluate_tool_call(
    platform: PolicyDecision,
    default_decision: PolicyDecision,
    rules: &[ToolPolicyRule],
    call: &ToolCall,
) -> ToolPolicyOutcome {
    if platform == PolicyDecision::Deny {
        return ToolPolicyOutcome::from_platform(PolicyDecision::Deny);
    }
    let strictest = |left: PolicyDecision, right: PolicyDecision| -> PolicyDecision {
        use PolicyDecision::*;
        match (left, right) {
            (Deny, _) | (_, Deny) => Deny,
            (RequireApproval, _) | (_, RequireApproval) => RequireApproval,
            _ => Allow,
        }
    };

    let mut matched: Vec<&ToolPolicyRule> = rules
        .iter()
        .filter(|rule| rule.enabled && matches(rule, call))
        .collect();
    // Stable ordering only affects which reason is reported for the same decision.
    matched.sort_by_key(|rule| (rule.priority, rule.name.clone()));

    let deny = matched
        .iter()
        .find(|rule| rule.action == ToolPolicyAction::Deny);
    let approval = matched
        .iter()
        .find(|rule| rule.action == ToolPolicyAction::RequireApproval);
    let allow = matched
        .iter()
        .find(|rule| rule.action == ToolPolicyAction::Allow);

    let (rule_decision, rule) = match (deny, approval, allow) {
        (Some(rule), _, _) => (PolicyDecision::Deny, Some(*rule)),
        (None, Some(rule), _) => (PolicyDecision::RequireApproval, Some(*rule)),
        (None, None, Some(rule)) => (PolicyDecision::Allow, Some(*rule)),
        (None, None, None) => (default_decision, None),
    };
    let decision = strictest(platform, rule_decision);
    let outcome = ToolPolicyOutcome {
        decision,
        rule_id: rule.map(|rule| rule.id),
        rule_name: rule.map(|rule| rule.name.clone()),
        reason: rule.and_then(|rule| rule.reason.clone()),
        impact: rule.and_then(|rule| rule.impact.clone()),
    };
    if decision == PolicyDecision::RequireApproval && outcome.reason.is_none() {
        ToolPolicyOutcome {
            reason: Some("platform requires approval for this effect".into()),
            ..outcome
        }
    } else {
        outcome
    }
}

/// Validate a rule before storing it. Returns a human-readable issue list.
pub fn validate_tool_policy(rule: &ToolPolicyRule) -> Vec<String> {
    let mut issues = Vec::new();
    if rule.name.trim().is_empty() {
        issues.push("name is required".into());
    }
    if rule.name.chars().count() > 120 {
        issues.push("name must be <= 120 characters".into());
    }
    for (field, value) in [
        ("matcher.tool", rule.matcher.tool.as_deref()),
        ("matcher.capability", rule.matcher.capability.as_deref()),
        ("matcher.connection", rule.matcher.connection.as_deref()),
        ("matcher.pathGlob", rule.matcher.path_glob.as_deref()),
    ] {
        if let Some(value) = value {
            if value.trim().is_empty() {
                issues.push(format!("{field} must not be empty"));
            }
        }
    }
    if let Err(error) = rule.matcher.compile() {
        issues.push(format!(
            "matcher.commandRegex is not a valid regex: {error}"
        ));
    }
    issues
}

fn matches(rule: &ToolPolicyRule, call: &ToolCall) -> bool {
    let matcher = &rule.matcher;
    if let Some(pattern) = &matcher.tool {
        if !glob_match(pattern, &call.tool) {
            return false;
        }
    }
    if let Some(pattern) = &matcher.capability {
        let Some(capability) = call.capability.as_deref() else {
            return false;
        };
        if !glob_match(pattern, capability) {
            return false;
        }
    }
    if let Some(effect) = matcher.effect {
        if call.effect != Some(effect) {
            return false;
        }
    }
    if let Some(pattern) = &matcher.connection {
        let Some(connection) = call.connection.as_deref() else {
            return false;
        };
        if !glob_match(pattern, connection) {
            return false;
        }
    }
    if let Some(pattern) = &matcher.path_glob {
        let paths = argument_paths(&call.arguments);
        if paths.is_empty() || !paths.iter().any(|path| path_matches(pattern, path)) {
            return false;
        }
    }
    if let Some(pattern) = &matcher.command_regex {
        let pattern = pattern.trim();
        if pattern.is_empty() {
            return false;
        }
        let Ok(regex) = Regex::new(pattern) else {
            return false;
        };
        let Some(command) = argument_command(&call.arguments) else {
            return false;
        };
        if !regex.is_match(&command) {
            return false;
        }
    }
    true
}

fn argument_command(arguments: &serde_json::Value) -> Option<String> {
    for key in ["command", "cmd", "script"] {
        let Some(value) = arguments.get(key) else {
            continue;
        };
        match value {
            serde_json::Value::String(command) => return Some(command.clone()),
            serde_json::Value::Array(parts) => {
                let parts: Vec<&str> = parts.iter().filter_map(|part| part.as_str()).collect();
                if !parts.is_empty() {
                    return Some(parts.join(" "));
                }
            }
            _ => {}
        }
    }
    None
}

fn argument_paths(arguments: &serde_json::Value) -> Vec<String> {
    const KEYS: [&str; 8] = [
        "path",
        "file_path",
        "filePath",
        "paths",
        "files",
        "target",
        "destination",
        "directory",
    ];
    let mut paths = Vec::new();
    for key in KEYS {
        let Some(value) = arguments.get(key) else {
            continue;
        };
        match value {
            serde_json::Value::String(path) => paths.push(path.clone()),
            serde_json::Value::Array(items) => {
                for item in items {
                    match item {
                        serde_json::Value::String(path) => paths.push(path.clone()),
                        serde_json::Value::Object(object) => {
                            if let Some(path) = object.get("path").and_then(|v| v.as_str()) {
                                paths.push(path.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            serde_json::Value::Object(object) => {
                if let Some(path) = object.get("path").and_then(|v| v.as_str()) {
                    paths.push(path.to_string());
                }
            }
            _ => {}
        }
    }
    paths
}

/// Glob match with `*` (any sequence, including `/`) and `?` (one character).
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let (mut p, mut t) = (0usize, 0usize);
    let (mut star, mut mark) = (None, 0usize);
    while t < text.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            mark = t;
            p += 1;
        } else if let Some(star_position) = star {
            p = star_position + 1;
            mark += 1;
            t = mark;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

fn path_matches(pattern: &str, path: &str) -> bool {
    let path = normalize_path(path);
    let pattern = pattern.trim();
    if glob_match(pattern, &path) {
        return true;
    }
    if pattern.starts_with('/') {
        return false;
    }
    // Relative patterns also match any component-aligned suffix, so `src/**` matches
    // `/workspace/src/lib.rs` and `*.env` matches `/repo/.env`.
    let components: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    for index in 0..components.len() {
        let suffix = components[index..].join("/");
        if glob_match(pattern, &suffix) {
            return true;
        }
    }
    false
}

/// Lexically normalize a path: collapse `.`/`..` and duplicate separators without
/// touching the filesystem.
fn normalize_path(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut stack: Vec<&str> = Vec::new();
    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if !stack.is_empty() {
                    stack.pop();
                }
            }
            other => stack.push(other),
        }
    }
    let joined = stack.join("/");
    if absolute {
        format!("/{joined}")
    } else if joined.is_empty() {
        ".".into()
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autonomy::PolicyDecision;

    fn rule(name: &str, action: ToolPolicyAction, matcher: ToolPolicyMatcher) -> ToolPolicyRule {
        ToolPolicyRule {
            id: Uuid::now_v7(),
            name: name.into(),
            priority: 100,
            enabled: true,
            action,
            matcher,
            reason: None,
            impact: None,
        }
    }

    fn call(tool: &str, arguments: serde_json::Value) -> ToolCall {
        ToolCall {
            tool: tool.into(),
            capability: None,
            effect: None,
            connection: None,
            arguments,
        }
    }

    #[test]
    fn denies_win_over_allow_and_approval() {
        let rules = vec![
            rule(
                "allow read",
                ToolPolicyAction::Allow,
                ToolPolicyMatcher {
                    tool: Some("Read".into()),
                    ..Default::default()
                },
            ),
            rule(
                "ask shell",
                ToolPolicyAction::RequireApproval,
                ToolPolicyMatcher {
                    tool: Some("Bash".into()),
                    ..Default::default()
                },
            ),
            rule(
                "deny rm",
                ToolPolicyAction::Deny,
                ToolPolicyMatcher {
                    command_regex: Some("rm\\s+-rf".into()),
                    ..Default::default()
                },
            ),
        ];
        let outcome = evaluate_tool_call(
            PolicyDecision::Allow,
            PolicyDecision::Allow,
            &rules,
            &call("Bash", serde_json::json!({ "command": "rm -rf /" })),
        );
        assert_eq!(outcome.decision, PolicyDecision::Deny);
        assert_eq!(outcome.rule_name.as_deref(), Some("deny rm"));
    }

    #[test]
    fn allow_rules_cannot_weaken_platform_approval() {
        let rules = vec![rule(
            "allow everything",
            ToolPolicyAction::Allow,
            ToolPolicyMatcher::default(),
        )];
        let outcome = evaluate_tool_call(
            PolicyDecision::RequireApproval,
            PolicyDecision::RequireApproval,
            &rules,
            &call("Write", serde_json::json!({})),
        );
        assert_eq!(outcome.decision, PolicyDecision::RequireApproval);
    }

    #[test]
    fn default_approval_rule_matches_all() {
        let rules = vec![rule(
            "ask all tools",
            ToolPolicyAction::RequireApproval,
            ToolPolicyMatcher::default(),
        )];
        let outcome = evaluate_tool_call(
            PolicyDecision::Allow,
            PolicyDecision::Allow,
            &rules,
            &call("Bash", serde_json::json!({ "command": "git status" })),
        );
        assert_eq!(outcome.decision, PolicyDecision::RequireApproval);
        assert_eq!(outcome.rule_name.as_deref(), Some("ask all tools"));
    }

    #[test]
    fn path_globs_normalize_traversal() {
        let matcher = ToolPolicyMatcher {
            path_glob: Some("secrets/**".into()),
            ..Default::default()
        };
        assert!(matches(
            &rule("p", ToolPolicyAction::Deny, matcher.clone()),
            &call(
                "Read",
                serde_json::json!({ "path": "/workspace/secrets/key.txt" })
            )
        ));
        assert!(matches(
            &rule("p", ToolPolicyAction::Deny, matcher.clone()),
            &call(
                "Read",
                serde_json::json!({ "path": "a/../secrets/../secrets/key.txt" })
            )
        ));
        assert!(!matches(
            &rule("p", ToolPolicyAction::Deny, matcher),
            &call("Read", serde_json::json!({ "path": "src/main.rs" }))
        ));
    }

    #[test]
    fn command_arrays_are_joined() {
        let matcher = ToolPolicyMatcher {
            command_regex: Some("^git push".into()),
            ..Default::default()
        };
        assert!(matches(
            &rule("push", ToolPolicyAction::RequireApproval, matcher),
            &call("Bash", serde_json::json!({ "command": ["git", "push"] }))
        ));
    }

    #[test]
    fn effects_and_capabilities_must_match_when_configured() {
        let matcher = ToolPolicyMatcher {
            capability: Some("issues.*".into()),
            effect: Some(CapabilityEffect::Write),
            ..Default::default()
        };
        let mut write = call("issues.create", serde_json::json!({}));
        write.capability = Some("issues.create".into());
        write.effect = Some(CapabilityEffect::Write);
        assert!(matches(
            &rule("w", ToolPolicyAction::Deny, matcher.clone()),
            &write
        ));
        write.effect = Some(CapabilityEffect::Read);
        assert!(!matches(
            &rule("w", ToolPolicyAction::Deny, matcher),
            &write
        ));
    }

    #[test]
    fn allow_rule_resolves_a_stricter_default() {
        let rules = vec![ToolPolicyRule {
            reason: Some("read-only command".into()),
            ..rule(
                "allow git status",
                ToolPolicyAction::Allow,
                ToolPolicyMatcher {
                    command_regex: Some("^git status$".into()),
                    ..Default::default()
                },
            )
        }];
        let bash_call = call("Bash", serde_json::json!({ "command": "git status" }));
        // Native tools default to require_approval; the explicit allow resolves it.
        let outcome = evaluate_tool_call(
            PolicyDecision::Allow,
            PolicyDecision::RequireApproval,
            &rules,
            &bash_call,
        );
        assert_eq!(outcome.decision, PolicyDecision::Allow);
        assert_eq!(outcome.rule_name.as_deref(), Some("allow git status"));
        assert_eq!(outcome.reason.as_deref(), Some("read-only command"));

        // A different command still requires approval.
        let outcome = evaluate_tool_call(
            PolicyDecision::Allow,
            PolicyDecision::RequireApproval,
            &rules,
            &call("Bash", serde_json::json!({ "command": "git push" })),
        );
        assert_eq!(outcome.decision, PolicyDecision::RequireApproval);
    }

    #[test]
    fn invalid_regex_is_rejected_by_validation() {
        let bad = rule(
            "bad",
            ToolPolicyAction::Deny,
            ToolPolicyMatcher {
                command_regex: Some("([".into()),
                ..Default::default()
            },
        );
        assert!(!validate_tool_policy(&bad).is_empty());
    }
}
