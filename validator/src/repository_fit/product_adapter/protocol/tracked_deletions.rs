use super::*;
use crate::context::planned_write_conflict_paths;

/// Refuses a fit write when its exact target was deleted or left unmerged in
/// the candidate. Existing-file ownership conflicts remain the inspection
/// kernel's responsibility; this closes the absent-path bypass only.
pub(crate) fn reject_dirty_write_overlap(
    context: &LiveContext,
    plan: &FitPlan,
) -> Result<(), FitAdapterError> {
    let requested = plan
        .all_mutations()
        .into_iter()
        .map(|mutation| std::path::PathBuf::from(mutation.path().as_str()))
        .collect::<Vec<_>>();
    let reads = context
        .begin_read_session()
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    let conflicts = planned_write_conflict_paths(&reads, &requested)
        .map_err(|_| adapter_error(AdapterErrorId::ContextStale))?;
    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(adapter_error(AdapterErrorId::PlanConflict))
    }
}
