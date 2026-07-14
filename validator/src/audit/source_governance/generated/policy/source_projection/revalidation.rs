use crate::audit::source_governance::GovernedSource;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) fn failures(
    root: &Path,
    paths: &BTreeSet<String>,
    sources: &BTreeMap<&str, &GovernedSource>,
) -> Vec<String> {
    let mut failures = Vec::new();
    for relative in paths {
        let Some(captured) = sources.get(relative.as_str()) else {
            failures.push(format!(
                "generated_source_projection_revalidation_source_missing:{relative}"
            ));
            continue;
        };
        match captured.revalidate(root) {
            Ok(()) => {}
            Err(error) if error.content_changed() => failures.push(format!(
                "generated_source_projection_changed_during_session:{relative}"
            )),
            Err(error) => failures.push(format!(
                "generated_source_projection_revalidation_failed:{relative}:{}",
                error.stable_text()
            )),
        }
    }
    failures
}
