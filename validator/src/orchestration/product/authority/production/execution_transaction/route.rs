use super::super::root_authority::{
    RootActionPermitVerification, RootAuthority, RootReconcilePermitVerification,
};
use super::super::{permit_id, ProductError, ProductionRootAuthority, RootOperation, RootPermit};
use crate::orchestration::product::command::RootActionRequest;
use crate::orchestration::product::runtime_adapter::{
    CurrentRuntimeView, OrchestrationRuntimeAdapter, RuntimeActionRequest, RuntimeActionSource,
};
use crate::orchestration::product::{
    journal_head_identity, ProductContext, ProductWorkspace, ReconcileOutcome, ReconcileRequest,
    RecoverOutcome, RecoverRequest, ResumeOutcome, ResumeRequest,
};

#[path = "reconcile.rs"]
mod reconcile;
#[path = "recover.rs"]
mod recover;
#[path = "resume.rs"]
mod resume;

enum ExecutionRequest<'a> {
    Resume {
        context: &'a ProductContext,
        workspace: &'a ProductWorkspace,
        permit: &'a RootPermit,
        source: RuntimeActionSource<'a>,
        action: &'a RootActionRequest,
        request: &'a ResumeRequest,
    },
    Recover {
        context: &'a ProductContext,
        workspace: &'a ProductWorkspace,
        permit: &'a RootPermit,
        source: RuntimeActionSource<'a>,
        action: &'a RootActionRequest,
        request: &'a RecoverRequest,
    },
    Reconcile {
        context: &'a ProductContext,
        workspace: &'a ProductWorkspace,
        permit: &'a RootPermit,
        view: &'a CurrentRuntimeView,
        action: &'a RootActionRequest,
        request: &'a ReconcileRequest,
    },
}

pub(crate) enum ProductionExecutionOutcome {
    Resume(ResumeOutcome),
    Recover(RecoverOutcome),
    Reconcile(ReconcileOutcome),
}

pub(in crate::orchestration::product::authority::production::sealed_authority) struct ValidatedExecution<
    'a,
> {
    permit_id: String,
    request: ExecutionRequest<'a>,
}

pub(in crate::orchestration::product::authority::production::sealed_authority) struct ReservedExecution<
    'a,
> {
    permit_id: String,
    request: ExecutionRequest<'a>,
}

impl<'a> ValidatedExecution<'a> {
    pub(in crate::orchestration::product::authority::production::sealed_authority) fn permit_id(
        &self,
    ) -> &str {
        &self.permit_id
    }

    pub(in crate::orchestration::product::authority::production::sealed_authority) fn reserve(
        self,
    ) -> ReservedExecution<'a> {
        ReservedExecution {
            permit_id: self.permit_id,
            request: self.request,
        }
    }
}

impl<'a> ReservedExecution<'a> {
    pub(in crate::orchestration::product::authority::production::sealed_authority) fn permit_id(
        &self,
    ) -> &str {
        &self.permit_id
    }

    pub(in crate::orchestration::product::authority::production::sealed_authority) fn execute(
        self,
    ) -> Result<ProductionExecutionOutcome, ProductError> {
        self.request.execute()
    }
}

impl<'a> ExecutionRequest<'a> {
    fn workspace(&self) -> &'a ProductWorkspace {
        match self {
            Self::Resume { workspace, .. }
            | Self::Recover { workspace, .. }
            | Self::Reconcile { workspace, .. } => workspace,
        }
    }

    fn permit(&self) -> &'a RootPermit {
        match self {
            Self::Resume { permit, .. }
            | Self::Recover { permit, .. }
            | Self::Reconcile { permit, .. } => permit,
        }
    }

    fn prevalidate(&self) -> Result<(), ProductError> {
        match self {
            Self::Resume {
                context,
                workspace,
                source,
                action,
                request,
                ..
            } => OrchestrationRuntimeAdapter::new(context, workspace)?.prevalidate_action(
                *source,
                action,
                &RuntimeActionRequest::Resume((*request).clone()),
            ),
            Self::Recover {
                context,
                workspace,
                source,
                action,
                request,
                ..
            } => OrchestrationRuntimeAdapter::new(context, workspace)?.prevalidate_action(
                *source,
                action,
                &RuntimeActionRequest::Recover((*request).clone()),
            ),
            Self::Reconcile {
                context,
                workspace,
                view,
                action,
                request,
                ..
            } => OrchestrationRuntimeAdapter::new(context, workspace)?
                .prevalidate_reconcile(view, action, request),
        }
    }
}

include!("route_verification.rs");
include!("route_execution.rs");
