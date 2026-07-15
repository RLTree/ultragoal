impl ExecutionRequest<'_> {
    fn verify(&self, authority: &RootAuthority) -> Result<(), ProductError> {
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

    fn execute(self) -> Result<ProductionExecutionOutcome, ProductError> {
        match self {
            Self::Resume {
                context,
                workspace,
                request,
                ..
            } => {
                resume::execute(context, workspace, request).map(ProductionExecutionOutcome::Resume)
            }
            Self::Recover {
                context,
                workspace,
                request,
                ..
            } => recover::execute(context, workspace, request)
                .map(ProductionExecutionOutcome::Recover),
            Self::Reconcile {
                context,
                workspace,
                request,
                ..
            } => reconcile::execute(context, workspace, request)
                .map(ProductionExecutionOutcome::Reconcile),
        }
    }
}
