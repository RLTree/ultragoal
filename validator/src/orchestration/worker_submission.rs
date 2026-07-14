use super::reconcile::result_commitment;
use super::{
    ArtifactWorkspace, EffectSink, EventKind, OrchestrationError, Orchestrator, WorkerResultV1,
};

impl<S: EffectSink> Orchestrator<S> {
    pub fn submit(
        &mut self,
        tick: u64,
        lease_id: &str,
        result: &WorkerResultV1,
        workspace: &ArtifactWorkspace,
    ) -> Result<(), OrchestrationError> {
        let (commitment, owner) = self.prepare_submission(lease_id, result, Some(workspace))?;
        let result_commitment_id = commitment.commitment_id()?;
        self.append(
            owner,
            tick,
            EventKind::WorkerSubmitted {
                lease_id: lease_id.to_owned(),
                commitment,
                result_commitment_id,
            },
        )
    }

    #[cfg(test)]
    pub(crate) fn submit_structural(
        &mut self,
        tick: u64,
        lease_id: &str,
        result: &WorkerResultV1,
    ) -> Result<(), OrchestrationError> {
        let (commitment, owner) = self.prepare_submission(lease_id, result, None)?;
        let result_commitment_id = commitment.commitment_id()?;
        self.append(
            owner,
            tick,
            EventKind::WorkerSubmitted {
                lease_id: lease_id.to_owned(),
                commitment,
                result_commitment_id,
            },
        )
    }

    fn prepare_submission(
        &self,
        lease_id: &str,
        result: &WorkerResultV1,
        workspace: Option<&ArtifactWorkspace>,
    ) -> Result<(super::event::ResultCommitment, super::Actor), OrchestrationError> {
        let runtime = self
            .projection
            .leases
            .get(lease_id)
            .ok_or(OrchestrationError::InvalidLease)?;
        let package = self
            .graph
            .package(&runtime.spec.node_id)
            .ok_or(OrchestrationError::UnknownNode)?;
        if let Some(workspace) = workspace {
            let verified = workspace.verify(result, &runtime.spec, package)?;
            if verified.result_id() != result.result_id()? {
                return Err(OrchestrationError::InvalidWorkerResult);
            }
        }
        let commitment = result_commitment(&self.binding, package, &runtime.spec, result)?;
        Ok((commitment, runtime.spec.owner.clone()))
    }
}
