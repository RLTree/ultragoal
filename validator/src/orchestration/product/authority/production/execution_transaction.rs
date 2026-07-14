use super::super::super::command::RootActionRequest;
use super::super::super::runtime_adapter::{
    CurrentRuntimeView, OrchestrationRuntimeAdapter, RuntimeActionRequest, RuntimeActionSource,
};
use super::super::super::{
    journal_head_identity, reconcile, recover, resume, ProductContext, ProductWorkspace,
    ReconcileOutcome, ReconcileRequest, RecoverOutcome, RecoverRequest, ResumeOutcome,
    ResumeRequest,
};
use super::super::{
    ExecutionAuthority, RootActionPermitVerification, RootAuthority,
    RootReconcilePermitVerification,
};
use super::{ProductError, RootOperation, RootPermit};

pub(super) enum ExecutionRequest<'a> {
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

impl<'a> ExecutionRequest<'a> {
    pub(super) fn workspace(&self) -> &'a ProductWorkspace {
        match self {
            Self::Resume { workspace, .. }
            | Self::Recover { workspace, .. }
            | Self::Reconcile { workspace, .. } => workspace,
        }
    }

    pub(super) fn prevalidate(&self) -> Result<(), ProductError> {
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

    pub(super) fn permit(&self) -> &'a RootPermit {
        match self {
            Self::Resume { permit, .. }
            | Self::Recover { permit, .. }
            | Self::Reconcile { permit, .. } => permit,
        }
    }

    pub(super) fn verify(&self, authority: &RootAuthority) -> Result<(), ProductError> {
        match self {
            Self::Resume {
                context,
                workspace,
                permit,
                request,
                ..
            } => authority.verify_action(RootActionPermitVerification {
                permit,
                expected_root: context.root(),
                operation: RootOperation::Resume,
                binding: context.binding(),
                workspace_identity: workspace.identity(),
                journal_head_identity: &journal_head_identity(&request.expected_head)?,
                tick: request.tick,
                target: &request.target,
            }),
            Self::Recover {
                context,
                workspace,
                permit,
                request,
                ..
            } => authority.verify_action(RootActionPermitVerification {
                permit,
                expected_root: context.root(),
                operation: RootOperation::Recover,
                binding: context.binding(),
                workspace_identity: workspace.identity(),
                journal_head_identity: &journal_head_identity(&request.expected_prior_head)?,
                tick: request.tick,
                target: &request.target,
            }),
            Self::Reconcile {
                context,
                workspace,
                permit,
                request,
                ..
            } => authority.verify_reconcile(RootReconcilePermitVerification {
                permit,
                expected_root: context.root(),
                binding: context.binding(),
                workspace_identity: workspace.identity(),
                journal_head_identity: &journal_head_identity(&request.expected_head)?,
                tick: request.tick,
                target: &request.target,
                resolution: &request.resolution,
            }),
        }
    }

    pub(super) fn execute(
        self,
        authority: &RootAuthority,
    ) -> Result<ProductionExecutionOutcome, ProductError> {
        match self {
            Self::Resume {
                context,
                workspace,
                request,
                ..
            } => resume(
                context,
                workspace,
                ExecutionAuthority::new(authority),
                request,
            )
            .map(ProductionExecutionOutcome::Resume),
            Self::Recover {
                context,
                workspace,
                request,
                ..
            } => recover(
                context,
                workspace,
                ExecutionAuthority::new(authority),
                request,
            )
            .map(ProductionExecutionOutcome::Recover),
            Self::Reconcile {
                context,
                workspace,
                request,
                ..
            } => reconcile(
                context,
                workspace,
                ExecutionAuthority::new(authority),
                request,
            )
            .map(ProductionExecutionOutcome::Reconcile),
        }
    }
}
