use super::*;

pub(super) fn reconcile_with_observe_roundtrip(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    command: &LiveLoopCommand,
    actual_work: &FullCommandRun,
    roundtrip: &mut dyn FnMut(
        query::RoundtripQuery,
        &str,
        &str,
    ) -> Result<query::ObserveReceipt, String>,
) -> Result<reconciliation_report::ReconciliationReport, String> {
    let observation_path = event::receipt_path(surface.id);
    let observation = event::write(
        root,
        surface,
        candidate,
        command,
        actual_work,
        &observation_path,
    )?;
    let run_id = observation.run_id.clone();
    let correlation_id = observation.correlation_id.clone();
    let logs = roundtrip(query::RoundtripQuery::Logs, &run_id, &correlation_id)?;
    let traces = roundtrip(query::RoundtripQuery::Traces, &run_id, &correlation_id)?;
    let metrics = roundtrip(query::RoundtripQuery::Metrics, &run_id, &correlation_id)?;
    let explain = roundtrip(
        query::RoundtripQuery::ExplainFailure,
        &run_id,
        &correlation_id,
    )?;
    Ok(reconciliation_report::from_roundtrips(
        observation,
        observation_path,
        logs,
        metrics,
        traces,
        explain,
    ))
}
