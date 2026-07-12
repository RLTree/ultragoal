use super::view::{CurrentRuntimeView, InterruptedRuntimeView, OrchestrationRuntimeAdapter};
use crate::orchestration::product::command::{RootActionReason, RootActionRequest};
use crate::orchestration::product::{
    ProductError, ReconcileOutcome, ReconcileRequest, RecoverOutcome, RecoverRequest,
    ResumeOutcome, ResumeRequest, RootAuthority, RootOperation, RootPermit, reconcile, recover,
    resume,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeActionRequest {
    Resume(ResumeRequest),
    Recover(RecoverRequest),
}

#[derive(Clone, Copy, Debug)]
pub enum RuntimeActionSource<'a> {
    Current(&'a CurrentRuntimeView),
    Interrupted(&'a InterruptedRuntimeView),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", content = "outcome", rename_all = "snake_case")]
pub enum RuntimeActionOutcome {
    Resume(ResumeOutcome),
    Recover(RecoverOutcome),
}

impl OrchestrationRuntimeAdapter<'_> {
    /// Routes an exact action-only root authorization. Reconciliation is kept
    /// on a separate interface because its permit binds the complete effect
    /// resolution rather than only the pre-decision action request.
    pub fn execute_action(
        &self,
        source: RuntimeActionSource<'_>,
        action: &RootActionRequest,
        authority: &RootAuthority,
        permit: &RootPermit,
        request: &RuntimeActionRequest,
    ) -> Result<RuntimeActionOutcome, ProductError> {
        match (source, request) {
            (RuntimeActionSource::Current(view), RuntimeActionRequest::Resume(request)) => {
                validate_resume(view, action, request)?;
                view.revalidate_exact(self.context, self.workspace)?;
                view.state().validate_action(self.workspace, action)?;
                resume(self.context, self.workspace, authority, permit, request)
                    .map(RuntimeActionOutcome::Resume)
            }
            (RuntimeActionSource::Interrupted(view), RuntimeActionRequest::Recover(request)) => {
                validate_recover(view, action, request)?;
                view.revalidate_exact(self.context, self.workspace)?;
                view.preview().validate_action(self.workspace, action)?;
                recover(self.context, self.workspace, authority, permit, request)
                    .map(RuntimeActionOutcome::Recover)
            }
            _ => Err(ProductError::AuthorityOperationMismatch),
        }
    }

    /// Executes one exact reconciliation whose supplied permit is verified by
    /// the product boundary against every byte of `request.resolution` before
    /// the journal can be mutated.
    pub fn execute_reconcile(
        &self,
        view: &CurrentRuntimeView,
        action: &RootActionRequest,
        authority: &RootAuthority,
        permit: &RootPermit,
        request: &ReconcileRequest,
    ) -> Result<ReconcileOutcome, ProductError> {
        validate_reconcile(view, action, request)?;
        view.revalidate_exact(self.context, self.workspace)?;
        view.state().validate_action(self.workspace, action)?;
        reconcile(self.context, self.workspace, authority, permit, request)
    }
}

fn validate_resume(
    view: &CurrentRuntimeView,
    action: &RootActionRequest,
    request: &ResumeRequest,
) -> Result<(), ProductError> {
    let source = view.inspection_request();
    if action.operation != RootOperation::Resume
        || action.reason != RootActionReason::RootInterrupted
        || request.expected_head != source.expected_head
        || request.expected_head != action.expected_head
        || request.tick != source.tick
        || request.live_workers != source.live_workers
        || request.target != action.target
    {
        return Err(ProductError::AuthorityOperationMismatch);
    }
    Ok(())
}

fn validate_reconcile(
    view: &CurrentRuntimeView,
    action: &RootActionRequest,
    request: &ReconcileRequest,
) -> Result<(), ProductError> {
    let source = view.inspection_request();
    if action.operation != RootOperation::Reconcile
        || action.reason != RootActionReason::EffectOutcomeAmbiguous
        || request.expected_head != source.expected_head
        || request.expected_head != action.expected_head
        || request.tick != source.tick
        || request.live_workers != source.live_workers
        || request.target != action.target
        || request.target.lease_id.as_deref() != Some(request.lease_id.as_str())
        || request.target.operation_id.as_deref() != Some(request.resolution.operation_id.as_str())
    {
        return Err(ProductError::AuthorityOperationMismatch);
    }
    Ok(())
}

fn validate_recover(
    view: &InterruptedRuntimeView,
    action: &RootActionRequest,
    request: &RecoverRequest,
) -> Result<(), ProductError> {
    let source = view.inspection_request();
    if action.operation != RootOperation::Recover
        || action.reason != RootActionReason::InterruptedPublication
        || request.expected_prior_head != source.expected_prior_head
        || request.expected_prior_head != action.expected_head
        || request.expected_event_id != source.expected_event_id
        || request.recovered_binding != source.recovered_binding
        || request.tick != source.tick
        || request.live_workers != source.live_workers
        || request.target != action.target
        || request.target.operation_id.as_deref() != Some(request.expected_event_id.as_str())
        || request.target.recovered_binding.as_ref() != Some(&request.recovered_binding)
    {
        return Err(ProductError::AuthorityOperationMismatch);
    }
    Ok(())
}
