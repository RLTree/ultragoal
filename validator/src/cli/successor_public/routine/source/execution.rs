use super::*;

pub(crate) fn execute_inner(
    root: &Path,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> Result<RuntimeOutcome, PublicFailure> {
    let options = source_context::options(invocation)?;
    let control = match options.interruption() {
        Some(source_context::ReservationInterruption::AfterReservation) => {
            PublicRoutineControl::InterruptAfterReservation
        }
        None => PublicRoutineControl::Run,
    };
    let target = source_context::target_root(root, &options)?;
    let discovery = source_context::discovery_context(&target)?;
    let manifest = manifest::load(&discovery, &target).map_err(PublicFailure::Manifest)?;
    let graph = manifest
        .graph()
        .map_err(|_| PublicFailure::Manifest(manifest::ManifestFailure::Invalid))?;
    let context = source_context::execution_context(&target, &manifest)?;
    validate_selected_sources(&context, &manifest)?;
    let snapshot = LocalDirtyTree::capture(&context).map_err(PublicFailure::Routine)?;
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine())
        .map_err(PublicFailure::Routine)?;
    let source_id = manifest.source_id();

    if plan.checks().is_empty() {
        let prepared = prepare(&context, &manifest, &graph, &snapshot, &plan)?;
        let result = mediate_public_routine_execution_with_control(
            None,
            &context,
            &plan,
            prepared,
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
            control,
            None,
        )
        .map_err(PublicFailure::Routine)?;
        return Ok(outcome::mediation(
            &result,
            terminal_event::mediation_context(&context, &plan, &graph, &snapshot, &source_id),
        ));
    }

    host_continuation::run(
        &target,
        &context,
        &manifest,
        &graph,
        &snapshot,
        &plan,
        &source_id,
        options.continuation(),
        control,
        home,
    )
}
