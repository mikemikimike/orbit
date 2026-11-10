//! Contracts shared across the control plane, workers and harness adapters.
//!
//! These types are deliberately independent of HTTP and SQL. Adapters implement the
//! traits here; the control plane only ever sees these structures.

pub mod connector;
pub mod events;
pub mod provider;
pub mod harness;

pub use connector::*;
pub use events::*;
pub use provider::*;
pub use harness::*;
