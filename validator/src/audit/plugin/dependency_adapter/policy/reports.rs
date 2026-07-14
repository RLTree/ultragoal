use super::super::model::DirectDependency;
use crate::audit::source_governance::GovernedInventory;
use crate::audit::source_governance::rust_syntax::{RustSyntaxReport, RustSyntaxRequest, analyze};
use std::collections::BTreeMap;

pub(super) fn syntax_reports(
    inventory: &GovernedInventory,
    failures: &mut Vec<String>,
) -> BTreeMap<String, RustSyntaxReport> {
    let mut reports = BTreeMap::new();
    for source in inventory
        .sources
        .iter()
        .filter(|source| source.relative.ends_with(".rs"))
    {
        match analyze(RustSyntaxRequest {
            source_path: &source.relative,
            source_bytes: &source.bytes,
            include_test_items: true,
        }) {
            Ok(report) => {
                reports.insert(source.relative.clone(), report);
            }
            Err(error) => failures.push(error.stable_text()),
        }
    }
    reports
}

pub(super) fn observed_sites<'a>(
    dependency: &DirectDependency,
    reports: &'a BTreeMap<String, RustSyntaxReport>,
) -> Vec<&'a String> {
    let crate_root = dependency.crate_name.replace('-', "_");
    reports
        .iter()
        .filter(|(_, report)| report.external_roots.contains(&crate_root))
        .map(|(path, _)| path)
        .collect()
}
