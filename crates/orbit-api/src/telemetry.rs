use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// Initialise structured logging. JSON output is used in production and when
/// `ORBIT_LOG_FORMAT=json`; human-readable logs are used in development.
///
/// OpenTelemetry export is enabled when `OTEL_EXPORTER_OTLP_ENDPOINT` is set (see
/// docs/architecture/overview.md). Traces are correlated with the request id and the
/// `traceparent` field carried by events.
pub fn init(service_name: &str) {
    let environment = std::env::var("ORBIT_ENV").unwrap_or_else(|_| "development".into());
    let json = std::env::var("ORBIT_LOG_FORMAT")
        .map(|v| v == "json")
        .unwrap_or(environment == "production");

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,orbit_api=info,orbit_worker=info,sqlx=warn"));

    let registry = tracing_subscriber::registry().with(filter);
    if json {
        let layer = tracing_subscriber::fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .flatten_event(true);
        registry.with(layer).init();
    } else {
        let layer = tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_thread_ids(false);
        registry.with(layer).init();
    }
    tracing::info!(
        service = service_name,
        environment = %environment,
        version = env!("CARGO_PKG_VERSION"),
        "telemetry initialised"
    );
}

/// Parse a boolean environment variable.
pub fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
        .unwrap_or(default)
}
