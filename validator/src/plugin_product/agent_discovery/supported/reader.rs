#[cfg(test)]
use super::report::{
    ReportState, SupportedHostAgentAuthorityReport, record_failure, reset_report, snapshot_report,
};
use super::roots::{SupportedHostAgentRoots, SupportedRootSet};
use super::transaction::SupportedTransaction;
use crate::plugin_product::agent_discovery::error::AgentDiscoveryError;
#[cfg(test)]
use crate::plugin_product::agent_discovery::error::AgentDiscoveryErrorId;
use crate::plugin_product::agent_discovery::host::{
    HostAgentAuthorityReader, HostAgentAuthorityRequest, HostAgentAuthorityTransaction,
    HostAgentAuthorityTransactionError,
};
use crate::plugin_product::agent_discovery::source::SourceAgentCatalog;
#[cfg(test)]
use std::sync::{Arc, Mutex};

/// Descriptor-anchored, read-only host authority reader.
pub struct SupportedHostAgentAuthorityReader {
    source: SourceAgentCatalog,
    roots: SupportedRootSet,
    #[cfg(test)]
    report: Arc<Mutex<ReportState>>,
    #[cfg(test)]
    after_transaction_open: Option<Box<dyn FnOnce()>>,
}

impl SupportedHostAgentAuthorityReader {
    pub fn open(
        source: SourceAgentCatalog,
        roots: SupportedHostAgentRoots,
    ) -> Result<Self, AgentDiscoveryError> {
        source.revalidate()?;
        let roots = SupportedRootSet::open(&roots)?;
        roots.revalidate()?;
        Ok(Self {
            source,
            roots,
            #[cfg(test)]
            report: Arc::new(Mutex::new(ReportState::default())),
            #[cfg(test)]
            after_transaction_open: None,
        })
    }

    #[cfg(test)]
    pub fn report(&self) -> SupportedHostAgentAuthorityReport {
        snapshot_report(&self.report)
    }

    #[cfg(test)]
    pub(crate) fn set_after_transaction_open_hook(&mut self, hook: impl FnOnce() + 'static) {
        self.after_transaction_open = Some(Box::new(hook));
    }
}

impl HostAgentAuthorityReader for SupportedHostAgentAuthorityReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostAgentAuthorityRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostAgentAuthorityTransaction, HostAgentAuthorityTransactionError>,
        ) -> T,
    ) -> T {
        #[cfg(test)]
        reset_report(&self.report, &self.source, request);
        if request.project_root_sha256() != self.source.project_root_sha256()
            || request.candidate_id() != self.source.candidate_id()
            || request.session_id() != self.source.session_id()
        {
            #[cfg(test)]
            record_failure(&self.report, AgentDiscoveryErrorId::InvalidBinding);
            return operation(Err(HostAgentAuthorityTransactionError::Failed));
        }
        #[cfg(test)]
        let transaction = SupportedTransaction::open(
            self.source.clone(),
            self.roots.clone(),
            request,
            Arc::clone(&self.report),
        );
        #[cfg(not(test))]
        let transaction =
            SupportedTransaction::open(self.source.clone(), self.roots.clone(), request);
        match transaction {
            Ok(mut transaction) => {
                #[cfg(test)]
                if let Some(hook) = self.after_transaction_open.take() {
                    hook();
                }
                operation(Ok(&mut transaction))
            }
            Err(_error) => {
                #[cfg(test)]
                record_failure(&self.report, _error.id());
                operation(Err(HostAgentAuthorityTransactionError::Failed))
            }
        }
    }
}
