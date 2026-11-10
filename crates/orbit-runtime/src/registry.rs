use std::collections::BTreeMap;
use std::sync::Arc;

use orbit_contracts::RuntimeDescriptor;
use orbit_domain::RuntimeKind;

use crate::{MockRuntimeAdapter, OpenCodeRuntimeAdapter, RuntimeAdapter};

/// Registry of adapter implementations available in this build.
#[derive(Clone)]
pub struct RuntimeRegistry {
    adapters: BTreeMap<RuntimeKind, Arc<dyn RuntimeAdapter>>,
}

impl RuntimeRegistry {
    pub fn builtin() -> Self {
        let adapters: Vec<Arc<dyn RuntimeAdapter>> = vec![
            Arc::new(MockRuntimeAdapter::default()),
            Arc::new(OpenCodeRuntimeAdapter),
            Arc::new(crate::planned::claude_code()),
            Arc::new(crate::PiRuntimeAdapter),
            Arc::new(crate::planned::generic()),
        ];
        Self {
            adapters: adapters
                .into_iter()
                .map(|adapter| (adapter.kind(), adapter))
                .collect(),
        }
    }

    pub fn get(&self, kind: RuntimeKind) -> Option<Arc<dyn RuntimeAdapter>> {
        self.adapters.get(&kind).cloned()
    }

    /// Like [`get`], but returns a typed error suitable for the worker.
    pub fn require(
        &self,
        kind: RuntimeKind,
    ) -> Result<Arc<dyn RuntimeAdapter>, orbit_contracts::RuntimeError> {
        self.get(kind).ok_or_else(|| {
            orbit_contracts::RuntimeError::new(
                orbit_contracts::RuntimeErrorKind::Unsupported,
                format!("runtime '{kind}' is not registered in this worker"),
            )
        })
    }

    pub fn descriptors(&self) -> Vec<RuntimeDescriptor> {
        self.adapters.values().map(|a| a.descriptor()).collect()
    }

    pub fn kinds(&self) -> Vec<RuntimeKind> {
        self.adapters.keys().copied().collect()
    }
}

impl Default for RuntimeRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}
