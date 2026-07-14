use super::super::command::{RootActionReason, RootActionRequest};
use super::super::{
    ProductError, ReconcileRequest, RecoverOutcome, RecoverRequest, ResumeOutcome, ResumeRequest,
    RootOperation,
};
use super::view::{CurrentRuntimeView, InterruptedRuntimeView, OrchestrationRuntimeAdapter};
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
    pub(crate) fn prevalidate_action(
        &self,
        source: RuntimeActionSource<'_>,
        action: &RootActionRequest,
        request: &RuntimeActionRequest,
    ) -> Result<(), ProductError> {
        match (source, request) {
            (RuntimeActionSource::Current(view), RuntimeActionRequest::Resume(request)) => {
                validate_resume(view, action, request)?;
                view.revalidate_exact(self.context, self.workspace)?;
                view.state().validate_action(self.workspace, action)?;
                Ok(())
            }
            (RuntimeActionSource::Interrupted(view), RuntimeActionRequest::Recover(request)) => {
                validate_recover(view, action, request)?;
                view.revalidate_exact(self.context, self.workspace)?;
                view.preview().validate_action(self.workspace, action)?;
                Ok(())
            }
            _ => Err(ProductError::AuthorityOperationMismatch),
        }
    }

    pub(crate) fn prevalidate_reconcile(
        &self,
        view: &CurrentRuntimeView,
        action: &RootActionRequest,
        request: &ReconcileRequest,
    ) -> Result<(), ProductError> {
        validate_reconcile(view, action, request)?;
        view.revalidate_exact(self.context, self.workspace)?;
        view.state().validate_action(self.workspace, action)?;
        request
            .resolution
            .validate_shape()
            .map_err(ProductError::from)?;
        Ok(())
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
