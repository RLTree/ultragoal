use crate::red::catalog::RedCatalogProjectionRequest;
use crate::schema_catalog::SchemaStore;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(crate) fn check(
    root: &Path,
    store: &SchemaStore,
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    let projection = match crate::red::catalog::check(RedCatalogProjectionRequest { root }) {
        Ok(value) => value,
        Err(error) => {
            push(failures, error.stable_text());
            return;
        }
    };
    let ids = projection
        .rows
        .iter()
        .map(|row| row.id.as_str())
        .collect::<BTreeSet<_>>();
    crate::audit::red::identity::check(root, store, &ids, failures);
}

pub(crate) fn law_guard_behavior_verified(root: &Path, id: &str, law: &str, field: &str) -> bool {
    // Repository-authored packets and receipts cannot prove that a production
    // law rejected a red stimulus. Keep this authority withheld until a
    // platform-backed observer can bind sealed production output.
    let _ = (root, id, law, field);
    false
}

fn push(failures: &mut BTreeMap<String, Vec<String>>, detail: impl Into<String>) {
    failures
        .entry("red-fixture-coverage".to_string())
        .or_default()
        .push(detail.into());
}
