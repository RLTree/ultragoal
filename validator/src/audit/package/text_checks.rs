use crate::schema_catalog::SchemaStore;
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn run(
    root: &Path,
    store: &SchemaStore,
    check_ids: &[String],
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    crate::audit::cli::self_law::append_package_text_checks(root, store, check_ids, failures);
}
