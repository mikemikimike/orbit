use std::collections::BTreeMap;
use std::sync::Arc;

use orbit_contracts::HarnessDescriptor;
use orbit_domain::HarnessKind;

use crate::{MockHarnessAdapter, OpenCodeHarnessAdapter, HarnessAdapter};

/// Registry of adapter implementations available in this build.
#[derive(Clone)]
pub struct HarnessRegistry {
    adapters: BTreeMap<HarnessKind, Arc<dyn HarnessAdapter>>,
}

impl HarnessRegistry {
    pub fn builtin() -> Self {
        let adapters: Vec<Arc<dyn HarnessAdapter>> = vec![
            Arc::new(MockHarnessAdapter::default()),
            Arc::new(OpenCodeHarnessAdapter),
            Arc::new(crate::planned::claude_code()),
            Arc::new(crate::PiHarnessAdapter),
            Arc::new(crate::planned::generic()),
        ];
        Self {
            adapters: adapters
                .into_iter()
                .map(|adapter| (adapter.kind(), adapter))
                .collect(),
        }
    }

    pub fn get(&self, kind: HarnessKind) -> Option<Arc<dyn HarnessAdapter>> {
        self.adapters.get(&kind).cloned()
    }

    /// Like [`get`], but returns a typed error suitable for the worker.
    pub fn require(
        &self,
        kind: HarnessKind,
    ) -> Result<Arc<dyn HarnessAdapter>, orbit_contracts::HarnessError> {
        self.get(kind).ok_or_else(|| {
            orbit_contracts::HarnessError::new(
                orbit_contracts::HarnessErrorKind::Unsupported,
                format!("harness '{kind}' is not registered in this worker"),
            )
        })
    }

    pub fn descriptors(&self) -> Vec<HarnessDescriptor> {
        self.adapters.values().map(|a| a.descriptor()).collect()
    }

    pub fn kinds(&self) -> Vec<HarnessKind> {
        self.adapters.keys().copied().collect()
    }
}

impl Default for HarnessRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}
