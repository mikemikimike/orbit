//! Event transport for ORBIT.
//!
//! PostgreSQL remains the durable source of truth for executions and events. The bus is
//! the low-latency notification channel; losing a bus message never loses work because
//! workers also poll the durable queue and a reaper re-publishes stale requests.

use std::pin::Pin;

use futures::Stream;
use orbit_contracts::{OrbitEvent, SubjectFilter};

pub mod memory;
pub mod nats;

pub use memory::MemoryEventBus;
pub use nats::NatsEventBus;

pub type EventStream = Pin<Box<dyn Stream<Item = Result<OrbitEvent, EventBusError>> + Send>>;

#[derive(Debug, thiserror::Error)]
pub enum EventBusError {
    #[error("event bus is unavailable: {0}")]
    Unavailable(String),
    #[error("event bus protocol error: {0}")]
    Protocol(String),
    #[error("invalid subject: {0}")]
    InvalidSubject(String),
    #[error("event bus configuration error: {0}")]
    Config(String),
    #[error("internal event bus error: {0}")]
    Internal(String),
}

#[async_trait::async_trait]
pub trait EventBus: Send + Sync {
    fn driver(&self) -> &'static str;
    /// Publish an event. Publish failures are returned so callers can decide whether the
    /// durable path (DB row + reaper) is sufficient.
    async fn publish(&self, event: OrbitEvent) -> Result<(), EventBusError>;
    async fn subscribe(&self, filter: SubjectFilter) -> Result<EventStream, EventBusError>;
    async fn health(&self) -> Result<(), EventBusError>;
}

pub struct EventBusConfig {
    pub driver: String,
    pub nats_url: String,
    pub stream_name: String,
    pub environment: String,
    pub memory_capacity: usize,
}

impl EventBusConfig {
    pub fn from_env() -> Result<Self, EventBusError> {
        Ok(Self {
            driver: std::env::var("ORBIT_EVENTS_DRIVER").unwrap_or_else(|_| "memory".into()),
            nats_url: std::env::var("ORBIT_NATS_URL")
                .unwrap_or_else(|_| "nats://localhost:4222".into()),
            stream_name: std::env::var("ORBIT_NATS_STREAM").unwrap_or_else(|_| "ORBIT".into()),
            environment: std::env::var("ORBIT_ENV").unwrap_or_else(|_| "development".into()),
            memory_capacity: std::env::var("ORBIT_EVENTS_MEMORY_CAPACITY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4096),
        })
    }
}

/// Build the configured bus. Refuses the non-durable memory driver in production.
pub async fn connect(
    config: &EventBusConfig,
) -> Result<std::sync::Arc<dyn EventBus>, EventBusError> {
    match config.driver.as_str() {
        "memory" => {
            if config.environment == "production" {
                return Err(EventBusError::Config(
                    "ORBIT_EVENTS_DRIVER=memory is not allowed in production; use nats".into(),
                ));
            }
            Ok(std::sync::Arc::new(MemoryEventBus::new(
                config.memory_capacity,
            )))
        }
        "nats" => Ok(std::sync::Arc::new(
            NatsEventBus::connect(&config.nats_url, &config.stream_name).await?,
        )),
        other => Err(EventBusError::Config(format!(
            "unknown ORBIT_EVENTS_DRIVER: {other}"
        ))),
    }
}
