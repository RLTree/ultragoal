use super::GovernedInventory;
use super::rust_syntax::{RustSyntaxRequest, analyze};

pub(super) fn failures(inventory: &GovernedInventory) -> Vec<String> {
    let mut failures = Vec::new();
    for source in inventory
        .sources
        .iter()
        .filter(|source| source.relative.ends_with(".rs"))
    {
        let report = match analyze(RustSyntaxRequest {
            source_path: &source.relative,
            source_bytes: &source.bytes,
            include_test_items: true,
        }) {
            Ok(report) => report,
            Err(error) => {
                failures.push(error.stable_text());
                continue;
            }
        };
        for lint in report.disallowed_lint_allowances {
            if broad_suppression(&lint) || production_path(&source.relative) {
                failures.push(format!(
                    "plugin_self_law_forbidden_lint_allowance:path={};lint={lint};repair=remove_suppression_and_correct_dead_or_untyped_boundary_code",
                    source.relative
                ));
            }
        }
        for detail in report.lint_metadata_failures {
            failures.push(format!(
                "plugin_self_law_lint_metadata_invalid:path={};detail={detail};repair=use_a_fully_parsed_allow_or_cfg_attr_attribute",
                source.relative
            ));
        }
    }
    failures.sort();
    failures.dedup();
    failures
}

fn broad_suppression(lint: &str) -> bool {
    matches!(
        lint,
        "dead_code" | "unreachable_code" | "unused" | "unused_imports" | "warnings"
    )
}

fn production_path(path: &str) -> bool {
    path.starts_with("validator/src/")
        && !path.contains("/self_tests/")
        && !path.contains("/tests/")
        && !path.ends_with("/tests.rs")
        && !path.ends_with("_tests.rs")
}
