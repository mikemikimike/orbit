//! Worker (execution-plane Pod/container) resource sizing.
//!
//! These are separate from agent execution limits (`timeoutSeconds`, `maxOutputTokens`):
//! they size the worker container that runs a harness, not a single execution.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct WorkerResources {
    pub requests: WorkerResourceQuantities,
    pub limits: WorkerResourceQuantities,
    pub workspace: WorkerWorkspace,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct WorkerResourceQuantities {
    /// Kubernetes CPU quantity, e.g. `100m` or `1`.
    pub cpu: Option<String>,
    /// Kubernetes memory quantity, e.g. `256Mi` or `1Gi`.
    pub memory: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct WorkerWorkspace {
    /// EmptyDir size limit for the worker workspace, e.g. `1Gi`.
    pub size_limit: Option<String>,
}

impl Default for WorkerWorkspace {
    fn default() -> Self {
        Self {
            size_limit: Some("1Gi".into()),
        }
    }
}

impl WorkerResources {
    /// Sensible defaults for a launched worker when a profile has no sizing.
    pub fn defaults() -> Self {
        Self {
            requests: WorkerResourceQuantities {
                cpu: Some("100m".into()),
                memory: Some("256Mi".into()),
            },
            limits: WorkerResourceQuantities {
                cpu: Some("1".into()),
                memory: Some("1Gi".into()),
            },
            workspace: WorkerWorkspace::default(),
        }
    }

    /// Fill missing quantities with defaults without overwriting explicit values.
    pub fn with_defaults(mut self) -> Self {
        let defaults = Self::defaults();
        self.requests.cpu = self.requests.cpu.or(defaults.requests.cpu);
        self.requests.memory = self.requests.memory.or(defaults.requests.memory);
        self.limits.cpu = self.limits.cpu.or(defaults.limits.cpu);
        self.limits.memory = self.limits.memory.or(defaults.limits.memory);
        if self.workspace.size_limit.is_none() {
            self.workspace.size_limit = defaults.workspace.size_limit;
        }
        self
    }

    pub fn is_empty(&self) -> bool {
        self.requests.cpu.is_none()
            && self.requests.memory.is_none()
            && self.limits.cpu.is_none()
            && self.limits.memory.is_none()
            && self.workspace.size_limit.is_none()
    }

    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        for (field, value) in [
            ("resources.requests.cpu", self.requests.cpu.as_deref()),
            ("resources.limits.cpu", self.limits.cpu.as_deref()),
        ] {
            if let Some(value) = value {
                if parse_cpu_millis(value).is_none() {
                    issues.push(format!("{field} must be a CPU quantity like 100m or 1"));
                }
            }
        }
        for (field, value) in [
            ("resources.requests.memory", self.requests.memory.as_deref()),
            ("resources.limits.memory", self.limits.memory.as_deref()),
            (
                "resources.workspace.sizeLimit",
                self.workspace.size_limit.as_deref(),
            ),
        ] {
            if let Some(value) = value {
                if parse_memory_bytes(value).is_none() {
                    issues.push(format!(
                        "{field} must be a memory quantity like 256Mi or 1Gi"
                    ));
                }
            }
        }
        if let (Some(request), Some(limit)) = (
            self.requests.cpu.as_deref().and_then(parse_cpu_millis),
            self.limits.cpu.as_deref().and_then(parse_cpu_millis),
        ) {
            if request > limit {
                issues.push("resources.requests.cpu must not exceed limits.cpu".into());
            }
        }
        if let (Some(request), Some(limit)) = (
            self.requests.memory.as_deref().and_then(parse_memory_bytes),
            self.limits.memory.as_deref().and_then(parse_memory_bytes),
        ) {
            if request > limit {
                issues.push("resources.requests.memory must not exceed limits.memory".into());
            }
        }
        issues
    }
}

/// Parse a Kubernetes CPU quantity into millicores.
pub fn parse_cpu_millis(value: &str) -> Option<u64> {
    let value = value.trim();
    if let Some(millis) = value.strip_suffix('m') {
        return millis.parse::<u64>().ok().filter(|millis| *millis > 0);
    }
    let cores = value.parse::<f64>().ok()?;
    if !cores.is_finite() || cores <= 0.0 {
        return None;
    }
    Some((cores * 1000.0).round() as u64)
}

/// Parse a Kubernetes memory quantity into bytes.
pub fn parse_memory_bytes(value: &str) -> Option<u64> {
    let value = value.trim();
    let units = [
        ("Ki", 1024u64),
        ("Mi", 1024u64.pow(2)),
        ("Gi", 1024u64.pow(3)),
        ("Ti", 1024u64.pow(4)),
        ("K", 1000),
        ("M", 1000u64.pow(2)),
        ("G", 1000u64.pow(3)),
        ("T", 1000u64.pow(4)),
    ];
    for (suffix, multiplier) in units {
        if let Some(number) = value.strip_suffix(suffix) {
            return number
                .parse::<u64>()
                .ok()
                .and_then(|number| number.checked_mul(multiplier));
        }
    }
    value.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_orders_quantities() {
        let resources = WorkerResources {
            requests: WorkerResourceQuantities {
                cpu: Some("500m".into()),
                memory: Some("512Mi".into()),
            },
            limits: WorkerResourceQuantities {
                cpu: Some("1".into()),
                memory: Some("1Gi".into()),
            },
            workspace: WorkerWorkspace {
                size_limit: Some("2Gi".into()),
            },
        };
        assert!(resources.validate().is_empty());
        assert_eq!(parse_cpu_millis("1"), Some(1000));
        assert_eq!(parse_cpu_millis("250m"), Some(250));
        assert_eq!(parse_memory_bytes("1Gi"), Some(1024 * 1024 * 1024));
    }

    #[test]
    fn rejects_inverted_limits() {
        let resources = WorkerResources {
            requests: WorkerResourceQuantities {
                cpu: Some("2".into()),
                memory: Some("2Gi".into()),
            },
            limits: WorkerResourceQuantities {
                cpu: Some("500m".into()),
                memory: Some("1Gi".into()),
            },
            ..Default::default()
        };
        let issues = resources.validate();
        assert_eq!(issues.len(), 2);
    }

    #[test]
    fn fills_defaults_without_overwriting() {
        let resources = WorkerResources {
            limits: WorkerResourceQuantities {
                memory: Some("2Gi".into()),
                ..Default::default()
            },
            ..Default::default()
        }
        .with_defaults();
        assert_eq!(resources.requests.cpu.as_deref(), Some("100m"));
        assert_eq!(resources.limits.memory.as_deref(), Some("2Gi"));
    }
}
