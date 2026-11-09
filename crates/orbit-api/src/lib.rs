//! ORBIT control-plane HTTP API.
//!
//! The API is a thin, well-typed layer over `orbit-store` and the event bus. It never
//! executes agent tasks; executions are queued durably and dispatched by `orbit-worker`.

pub mod audit;
pub mod auth;
pub mod bootstrap;
pub mod config;
pub mod connectors;
pub mod dto;
pub mod error;
pub mod extract;
pub mod handlers;
pub mod metrics;
pub mod openapi;
pub mod request_id;
pub mod routes;
pub mod state;
pub mod telemetry;

pub use bootstrap::{initialize, Initialized};
pub use config::ApiConfig;
pub use routes::build_router;
pub use state::{AppState, DevIdentity};
