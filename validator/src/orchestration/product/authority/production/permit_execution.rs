struct ValidatedExecution<'a> {
    permit_id: String,
    request: ExecutionRequest<'a>,
}

struct ReservedExecution<'a> {
    permit_id: String,
    request: ExecutionRequest<'a>,
}

impl<'a> ValidatedExecution<'a> {
    fn into_parts(self) -> (String, ExecutionRequest<'a>) {
        (self.permit_id, self.request)
    }
}

impl<'a> ReservedExecution<'a> {
    fn new(permit_id: String, request: ExecutionRequest<'a>) -> Self {
        Self { permit_id, request }
    }

    fn permit_id(&self) -> &str {
        &self.permit_id
    }

    fn execute(
        self,
        authority: &RootAuthority,
    ) -> Result<ProductionExecutionOutcome, ProductError> {
        self.request.execute(authority)
    }
}

impl ProductionRootAuthority {
    pub(crate) fn execute_action<'a>(
        &'a self,
        context: &'a super::super::ProductContext,
        workspace: &'a super::super::ProductWorkspace,
        source: super::super::runtime_adapter::RuntimeActionSource<'a>,
        action: &'a super::super::command::RootActionRequest,
        permit: &'a RootPermit,
        request: &'a super::super::runtime_adapter::RuntimeActionRequest,
    ) -> Result<ProductionExecutionOutcome, ProductError> {
        let execution = match request {
            super::super::runtime_adapter::RuntimeActionRequest::Resume(request) => {
                ExecutionRequest::Resume {
                    context,
                    workspace,
                    permit,
                    source,
                    action,
                    request,
                }
            }
            super::super::runtime_adapter::RuntimeActionRequest::Recover(request) => {
                ExecutionRequest::Recover {
                    context,
                    workspace,
                    permit,
                    source,
                    action,
                    request,
                }
            }
        };
        self.execute(execution)
    }

    pub(crate) fn execute_reconcile<'a>(
        &'a self,
        context: &'a super::super::ProductContext,
        workspace: &'a super::super::ProductWorkspace,
        view: &'a super::super::runtime_adapter::CurrentRuntimeView,
        action: &'a super::super::command::RootActionRequest,
        permit: &'a RootPermit,
        request: &'a super::super::ReconcileRequest,
    ) -> Result<ProductionExecutionOutcome, ProductError> {
        self.execute(ExecutionRequest::Reconcile {
            context,
            workspace,
            permit,
            view,
            action,
            request,
        })
    }

    fn execute<'a>(
        &'a self,
        request: ExecutionRequest<'a>,
    ) -> Result<ProductionExecutionOutcome, ProductError> {
        let permit_id = permit_id(request.permit())?;
        self.ledger.require_issued(&permit_id)?;
        let workspace = request.workspace();
        workspace.verify()?;
        let reservation = crate::orchestration::FileJournal::with_existing_exclusive_lock(
            workspace.root(),
            || {
                workspace.verify()?;
                request.prevalidate()?;
                request.verify(&self.authority)?;
                let execution = ValidatedExecution { permit_id, request };
                self.ledger.require_issued(&execution.permit_id)?;
                self.ledger.reserve(execution)
            },
        )?;
        self.ledger.complete(reservation, &self.authority)
    }
}
