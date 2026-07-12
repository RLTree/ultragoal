#[cfg(unix)]
mod sys;
#[cfg(unix)]
mod unix;

use super::{OrchestrationError, RootIntegrationIntent, RootIntegrationObservation};
use std::path::Path;

/// Anchored, read-only observer for root integration paths. Opening or
/// observing never creates, locks, or mutates a workspace object.
pub struct RootWorkspace {
    #[cfg(unix)]
    inner: unix::Workspace,
}

impl RootWorkspace {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, OrchestrationError> {
        #[cfg(unix)]
        {
            return unix::Workspace::open(root.as_ref()).map(|inner| Self { inner });
        }
        #[cfg(not(unix))]
        {
            let _ = root;
            Err(OrchestrationError::IntegrationAmbiguous)
        }
    }

    pub(crate) fn observe(
        &self,
        intent: &RootIntegrationIntent,
    ) -> Result<RootIntegrationObservation, OrchestrationError> {
        #[cfg(unix)]
        {
            return self.inner.observe(intent);
        }
        #[cfg(not(unix))]
        {
            let _ = intent;
            Err(OrchestrationError::IntegrationAmbiguous)
        }
    }
}
