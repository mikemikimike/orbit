use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Minimal, dependency-free metrics registry exposed at `/metrics`.
///
/// A full OpenTelemetry pipeline is configured by the binaries; this endpoint provides
/// stable, low-cardinality counters for Kubernetes dashboards and alerts.
#[derive(Debug, Default)]
pub struct Metrics {
    http_requests: AtomicU64,
    http_errors: AtomicU64,
    executions_created: AtomicU64,
    agents_created: AtomicU64,
    provider_probes: AtomicU64,
    provider_probe_failures: AtomicU64,
    started_at: std::sync::OnceLock<chrono::DateTime<chrono::Utc>>,
}

impl Metrics {
    pub fn new() -> Self {
        let metrics = Self::default();
        let _ = metrics.started_at.set(chrono::Utc::now());
        metrics
    }

    pub fn inc_requests(&self) {
        self.http_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_errors(&self) {
        self.http_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_executions(&self) {
        self.executions_created.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_agents(&self) {
        self.agents_created.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_probe(&self, ok: bool) {
        self.provider_probes.fetch_add(1, Ordering::Relaxed);
        if !ok {
            self.provider_probe_failures.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn render(&self, extra: &BTreeMap<String, String>) -> String {
        let mut out = String::new();
        let uptime = self
            .started_at
            .get()
            .map(|t| (chrono::Utc::now() - *t).num_seconds())
            .unwrap_or(0);
        out.push_str("# HELP orbit_http_requests_total Total HTTP requests handled\n");
        out.push_str("# TYPE orbit_http_requests_total counter\n");
        out.push_str(&format!(
            "orbit_http_requests_total {}\n",
            self.http_requests.load(Ordering::Relaxed)
        ));
        out.push_str("# HELP orbit_http_errors_total HTTP requests answered with an error\n");
        out.push_str("# TYPE orbit_http_errors_total counter\n");
        out.push_str(&format!(
            "orbit_http_errors_total {}\n",
            self.http_errors.load(Ordering::Relaxed)
        ));
        out.push_str("# HELP orbit_executions_created_total Executions accepted\n");
        out.push_str("# TYPE orbit_executions_created_total counter\n");
        out.push_str(&format!(
            "orbit_executions_created_total {}\n",
            self.executions_created.load(Ordering::Relaxed)
        ));
        out.push_str("# HELP orbit_agents_created_total Agents created\n");
        out.push_str("# TYPE orbit_agents_created_total counter\n");
        out.push_str(&format!(
            "orbit_agents_created_total {}\n",
            self.agents_created.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "orbit_provider_probes_total {}\n",
            self.provider_probes.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "orbit_provider_probe_failures_total {}\n",
            self.provider_probe_failures.load(Ordering::Relaxed)
        ));
        out.push_str("# HELP orbit_uptime_seconds Process uptime\n");
        out.push_str("# TYPE orbit_uptime_seconds gauge\n");
        out.push_str(&format!("orbit_uptime_seconds {uptime}\n"));
        for (key, value) in extra {
            out.push_str(&format!("orbit_info{{{key}=\"{value}\"}} 1\n"));
        }
        out
    }
}
