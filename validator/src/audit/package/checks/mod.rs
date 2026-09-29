use crate::scheduler::SchedulerConfig;
use crate::schema_catalog::SchemaStore;
use std::collections::BTreeMap;
use std::path::Path;

mod scheduled;

pub struct CheckResults {
    pub failures: BTreeMap<String, Vec<String>>,
}

pub fn checks_with_scheduler(
    root: &Path,
    store: &SchemaStore,
    check_ids: &[String],
    scheduler: SchedulerConfig,
) -> CheckResults {
    let mut failures = check_ids
        .iter()
        .map(|id| (id.clone(), Vec::new()))
        .collect::<BTreeMap<_, _>>();
    let schema = crate::audit::package::schema::validation::mapped(root, store, scheduler);
    failures
        .entry("schema-valid".to_string())
        .or_default()
        .extend(schema.failures);
    scheduled::package_checks(root, store, check_ids, scheduler, &mut failures);
    CheckResults { failures }
}

fn inventory_checks(root: &Path, failures: &mut BTreeMap<String, Vec<String>>) {
    crate::audit::cli::self_law::append_inventory_checks(root, failures);
}
