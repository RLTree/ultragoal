use super::{
    EffectSink, EventKind, IntegrationDisposition, OrchestrationError, Orchestrator,
    RootIntegrationIntent, RootIntegrationReceipt, RootWorkspace,
};

impl<S: EffectSink> Orchestrator<S> {
    pub fn begin_integration(
        &mut self,
        tick: u64,
        intent: RootIntegrationIntent,
    ) -> Result<(), OrchestrationError> {
        if self.journal.is_none() {
            return Err(OrchestrationError::IntegrationAmbiguous);
        }
        self.append(
            self.root.clone(),
            tick,
            EventKind::RootIntegrationStarted { intent },
        )
    }

    pub fn observe_integration(
        &mut self,
        tick: u64,
        workspace: &RootWorkspace,
    ) -> Result<IntegrationDisposition, OrchestrationError> {
        let intent = self
            .projection
            .integration_intent
            .as_ref()
            .ok_or(OrchestrationError::IntegrationAmbiguous)?
            .clone();
        let observation = workspace.observe(&intent)?;
        let disposition = observation.disposition();
        self.append(
            self.root.clone(),
            tick,
            EventKind::RootIntegrationObserved { observation },
        )?;
        Ok(disposition)
    }

    pub fn complete(
        &mut self,
        tick: u64,
        workspace: &RootWorkspace,
        integration: RootIntegrationReceipt,
    ) -> Result<(), OrchestrationError> {
        let intent = self
            .projection
            .integration_intent
            .as_ref()
            .ok_or(OrchestrationError::IntegrationAmbiguous)?
            .clone();
        let first = workspace.observe(&intent)?;
        if first.disposition() != IntegrationDisposition::Full {
            return Err(OrchestrationError::IntegrationAmbiguous);
        }
        let observation = workspace.observe(&intent)?;
        if observation != first || observation.disposition() != IntegrationDisposition::Full {
            return Err(OrchestrationError::IntegrationAmbiguous);
        }
        self.append(
            self.root.clone(),
            tick,
            EventKind::CandidateRebound {
                observation,
                integration,
            },
        )
    }
}
