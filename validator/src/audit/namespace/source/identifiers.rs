use crate::audit::source_governance::rust_syntax::{RustSyntaxRequest, analyze};
use crate::audit::source_governance::{GovernedInventory, GovernedSource};
#[cfg(test)]
use std::collections::BTreeSet;
#[cfg(test)]
use std::path::Path;

#[cfg(test)]
pub(crate) fn failures(root: &Path, actual_files: &[String]) -> Vec<String> {
    let allowed = actual_files
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    match crate::audit::source_governance::capture(root) {
        Ok(inventory) => failures_for_sources(
            inventory
                .sources
                .iter()
                .filter(|source| allowed.contains(source.relative.as_str())),
        ),
        Err(failures) => failures,
    }
}

pub(crate) fn failures_for_inventory(inventory: &GovernedInventory) -> Vec<String> {
    failures_for_sources(inventory.sources.iter())
}

fn failures_for_sources<'a>(sources: impl Iterator<Item = &'a GovernedSource>) -> Vec<String> {
    let mut failures = Vec::new();
    for source in sources.filter(|source| source.relative.ends_with(".rs")) {
        let report = match analyze(RustSyntaxRequest {
            source_path: &source.relative,
            source_bytes: &source.bytes,
            include_test_items: true,
        }) {
            Ok(value) => value,
            Err(error) => {
                failures.push(error.stable_text());
                continue;
            }
        };
        for declaration in report.identifiers {
            if let Some((check_id, label, why, repair)) = classification(&declaration.name) {
                failures.push(format!(
                    "{check_id}:path={};identifier={};label={label};why={why};repair={repair};claims=completion,review,package,readiness,release,product_readiness,cli_self_law,source_audit,final_packet,update_goal;exception_allowed=false",
                    source.relative, declaration.name
                ));
            }
        }
        let source_text = String::from_utf8_lossy(&source.bytes);
        failures.extend(source_text.lines().enumerate().filter_map(|(index, line)| {
            super::string_labels::failure(&source.relative, index + 1, line)
        }));
        failures.extend(super::string_labels::raw_source_failures(
            &source.relative,
            &source_text,
        ));
    }
    failures.sort();
    failures.dedup();
    failures
}

fn classification(
    identifier: &str,
) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    if let Some(label) = super::path_labels::product_opaque_goal_work_label(identifier) {
        return Some((
            "namespace_validator_source_product_opaque_goal_work_identifier",
            label,
            "identifier_names_goal_work_or_evidence_posture_instead_of_product_behavior",
            "rename_identifier_by_cli_product_behavior",
        ));
    }
    super::path_labels::generic_identifier_bucket_label(identifier).map(|label| {
        (
            "namespace_validator_source_generic_identifier",
            label,
            "identifier_names_generic_bucket_or_helper_posture_instead_of_product_behavior",
            "rename_identifier_by_the_product_behavior_or_domain_contract_it_serves",
        )
    })
}
