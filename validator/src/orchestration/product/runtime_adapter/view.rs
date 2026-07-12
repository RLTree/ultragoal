use crate::orchestration::product::command::{
    self, CommandProjection, InterruptedRecoveryRequest, InterruptedRecoveryView,
    OrchestrationStateRequest, OrchestrationStateView,
};
use crate::orchestration::product::{ProductContext, ProductError, ProductWorkspace};

/// Process-local adapter bound to one root-supplied context and anchored
/// durable workspace.
#[derive(Debug)]
pub struct OrchestrationRuntimeAdapter<'a> {
    pub(super) context: &'a ProductContext,
    pub(super) workspace: &'a ProductWorkspace,
}

/// A sealed current-state inspection together with the exact tick and live
/// worker set that produced it. It deliberately has no serialization form.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentRuntimeView {
    request: OrchestrationStateRequest,
    state: OrchestrationStateView,
}

/// A sealed interrupted-publication inspection together with the exact source
/// request that produced it. It deliberately has no serialization form.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterruptedRuntimeView {
    request: InterruptedRecoveryRequest,
    preview: InterruptedRecoveryView,
}

impl<'a> OrchestrationRuntimeAdapter<'a> {
    pub fn new(
        context: &'a ProductContext,
        workspace: &'a ProductWorkspace,
    ) -> Result<Self, ProductError> {
        workspace.verify()?;
        Ok(Self { context, workspace })
    }

    /// Reads and seals the exact current orchestration state without writing.
    pub fn inspect_current(
        &self,
        request: &OrchestrationStateRequest,
    ) -> Result<CurrentRuntimeView, ProductError> {
        self.workspace.verify()?;
        let state = command::inspect(self.context, self.workspace, request)?;
        let view = CurrentRuntimeView {
            request: request.clone(),
            state,
        };
        view.revalidate_exact(self.context, self.workspace)?;
        Ok(view)
    }

    /// Reads and seals one exact interrupted append without publishing it.
    pub fn inspect_interrupted(
        &self,
        request: &InterruptedRecoveryRequest,
    ) -> Result<InterruptedRuntimeView, ProductError> {
        self.workspace.verify()?;
        let preview = command::inspect_interrupted(self.context, self.workspace, request)?;
        let view = InterruptedRuntimeView {
            request: request.clone(),
            preview,
        };
        view.revalidate_exact(self.context, self.workspace)?;
        Ok(view)
    }

    /// Produces bounded deterministic bytes from a trusted current view. Both
    /// the source and resulting projection are revalidated in the same call.
    pub fn project_current(
        &self,
        view: &CurrentRuntimeView,
        projection: &CommandProjection,
    ) -> Result<Option<Vec<u8>>, ProductError> {
        view.revalidate_exact(self.context, self.workspace)?;
        let bytes = command::project(&view.state, self.workspace, projection)?;
        view.revalidate_exact(self.context, self.workspace)?;
        Ok(bytes)
    }
}

impl CurrentRuntimeView {
    pub fn state(&self) -> &OrchestrationStateView {
        &self.state
    }

    pub fn inspection_request(&self) -> &OrchestrationStateRequest {
        &self.request
    }

    pub(super) fn revalidate_exact(
        &self,
        context: &ProductContext,
        workspace: &ProductWorkspace,
    ) -> Result<(), ProductError> {
        self.state.revalidate(workspace)?;
        let fresh = command::inspect(context, workspace, &self.request)?;
        if fresh != self.state {
            return Err(ProductError::AuthorityInvalid);
        }
        self.state.revalidate(workspace)
    }
}

impl InterruptedRuntimeView {
    pub fn preview(&self) -> &InterruptedRecoveryView {
        &self.preview
    }

    pub fn inspection_request(&self) -> &InterruptedRecoveryRequest {
        &self.request
    }

    pub(super) fn revalidate_exact(
        &self,
        context: &ProductContext,
        workspace: &ProductWorkspace,
    ) -> Result<(), ProductError> {
        self.preview
            .validate_action(workspace, &self.preview.root_action_request)?;
        let fresh = command::inspect_interrupted(context, workspace, &self.request)?;
        if fresh != self.preview {
            return Err(ProductError::AuthorityInvalid);
        }
        self.preview
            .validate_action(workspace, &self.preview.root_action_request)
    }
}
