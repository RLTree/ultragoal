impl ProductionRootAuthority {
    pub(crate) fn execute_action<'a>(
        &'a self,
        context: &'a ProductContext,
        workspace: &'a ProductWorkspace,
        source: RuntimeActionSource<'a>,
        action: &'a RootActionRequest,
        permit: &'a RootPermit,
        request: &'a RuntimeActionRequest,
    ) -> Result<ProductionExecutionOutcome, ProductError> {
        let execution = match request {
            RuntimeActionRequest::Resume(request) => ExecutionRequest::Resume {
                context,
                workspace,
                permit,
                source,
                action,
                request,
            },
            RuntimeActionRequest::Recover(request) => ExecutionRequest::Recover {
                context,
                workspace,
                permit,
                source,
                action,
                request,
            },
        };
        self.execute(execution)
    }

    pub(crate) fn execute_reconcile<'a>(
        &'a self,
        context: &'a ProductContext,
        workspace: &'a ProductWorkspace,
        view: &'a CurrentRuntimeView,
        action: &'a RootActionRequest,
        permit: &'a RootPermit,
        request: &'a ReconcileRequest,
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
                self.ledger.require_issued(execution.permit_id())?;
                self.ledger.reserve(execution)
            },
        )?;
        self.ledger.complete(reservation)
    }
}
