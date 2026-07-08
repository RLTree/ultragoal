use super::ValidationResult;
use crate::scheduler::SchedulerConfig;
use std::path::Path;

pub(super) fn validate(
    root: &Path,
    store: &crate::schema_catalog::SchemaStore,
    scheduler: SchedulerConfig,
) -> Result<ValidationResult, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let inputs = crate::cli::live_loop::changed_inputs::ChangedInputs::collect(
        root,
        &candidate,
        "hot",
        "verified-local",
    );
    let paths = inputs.surface_changed_paths("schema_validation");
    if paths.iter().any(|path| path.starts_with("schemas/")) {
        return Ok(super::mapped(root, store, scheduler));
    }
    let results = crate::audit::package::checks::schema_validation_results_for_paths(
        root, store, scheduler, &paths,
    );
    Ok(ValidationResult {
        failures: results.failures,
        scheduler_metrics: results.scheduler_metrics,
    })
}
