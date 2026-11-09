//! Security primitives: authentication abstractions, permission checks, secret custody.
//!
//! This crate intentionally contains no SQL and no HTTP handlers. The persistence layer
//! implements [`SecretStore`] and credential lookups; the API layer wires extractors.

pub mod auth;
pub mod error;
pub mod permissions;
pub mod secrets;

pub use auth::*;
pub use error::*;
pub use permissions::*;
pub use secrets::*;
