//! Runtime adapters and the runtime registry.
//!
//! See `docs/architecture/runtime-contract.md`. Adapters never touch PostgreSQL, HTTP or
//! the frontend. The control plane only stores their descriptors and capabilities; the
//! worker executes them.

pub mod mock;
pub mod opencode;
pub mod pi;
pub mod planned;
pub mod registry;
pub mod testing;

pub use mock::MockRuntimeAdapter;
pub use opencode::OpenCodeRuntimeAdapter;
pub use pi::PiRuntimeAdapter;
pub use planned::DescriptorOnlyAdapter;
pub use registry::RuntimeRegistry;

pub use orbit_contracts::{
    CreateInstanceRequest, ExecutionOutcome, ExecutionRequest, InstanceHealth, RuntimeAdapter,
    RuntimeCapabilities, RuntimeConfig, RuntimeDescriptor, RuntimeError, RuntimeErrorKind,
    RuntimeEvent, RuntimeEventKind, RuntimeHealth, RuntimeInstance, RuntimeInstanceRef,
    RuntimeInstanceStatus,
};
