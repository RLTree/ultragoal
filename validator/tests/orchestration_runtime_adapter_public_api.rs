use ultragoal::orchestration::product::command::{
    CommandProjection, InterruptedRecoveryRequest, OrchestrationStateRequest, RootActionRequest,
};
use ultragoal::orchestration::product::runtime_adapter::{
    CurrentRuntimeView, InterruptedRuntimeView, OrchestrationRuntimeAdapter, RuntimeActionOutcome,
    RuntimeActionRequest, RuntimeActionSource,
};
use ultragoal::orchestration::product::{
    ProductError, ProductionRootAuthority, ReconcileOutcome, ReconcileRequest, RootPermit,
};

#[test]
fn library_surface_is_typed_without_exposing_authority_construction() {
    fn inspect_current<'a>(
        adapter: &OrchestrationRuntimeAdapter<'a>,
        request: &OrchestrationStateRequest,
    ) -> Result<CurrentRuntimeView, ProductError> {
        adapter.inspect_current(request)
    }
    fn inspect_interrupted<'a>(
        adapter: &OrchestrationRuntimeAdapter<'a>,
        request: &InterruptedRecoveryRequest,
    ) -> Result<InterruptedRuntimeView, ProductError> {
        adapter.inspect_interrupted(request)
    }
    fn project<'a>(
        adapter: &OrchestrationRuntimeAdapter<'a>,
        view: &CurrentRuntimeView,
        projection: &CommandProjection,
    ) -> Result<Option<Vec<u8>>, ProductError> {
        adapter.project_current(view, projection)
    }
    fn execute_action<'a>(
        adapter: &OrchestrationRuntimeAdapter<'a>,
        source: RuntimeActionSource<'_>,
        action: &RootActionRequest,
        authority: &ProductionRootAuthority,
        permit: &RootPermit,
        request: &RuntimeActionRequest,
    ) -> Result<RuntimeActionOutcome, ProductError> {
        adapter.execute_production_action(authority, source, action, permit, request)
    }
    fn execute_reconcile<'a>(
        adapter: &OrchestrationRuntimeAdapter<'a>,
        current: &CurrentRuntimeView,
        action: &RootActionRequest,
        authority: &ProductionRootAuthority,
        permit: &RootPermit,
        request: &ReconcileRequest,
    ) -> Result<ReconcileOutcome, ProductError> {
        adapter.execute_production_reconcile(authority, current, action, permit, request)
    }

    let _ = inspect_current;
    let _ = inspect_interrupted;
    let _ = project;
    let _ = execute_action;
    let _ = execute_reconcile;
    assert!(
        std::any::type_name::<OrchestrationRuntimeAdapter<'static>>()
            .ends_with("runtime_adapter::view::OrchestrationRuntimeAdapter<'_>")
    );
}
