mod policy;
mod registry;

#[cfg(test)]
use crate::audit::source_governance::GovernedSource;
use crate::audit::source_governance::production_source::ProductionSource;
use crate::audit::source_governance::rust_syntax::{RustSyntaxReport, RustSyntaxRequest, analyze};
use std::collections::BTreeMap;

pub(crate) use registry::BoundaryRow;

pub(super) fn failures(sources: &[ProductionSource]) -> Vec<String> {
    failures_for_inputs(
        sources
            .iter()
            .map(|source| (source.relative.as_str(), source.bytes.as_slice())),
        registry::BOUNDARY_ROWS,
    )
}

#[cfg(test)]
pub(crate) fn failures_for_sources_and_rows<'a>(
    sources: impl Iterator<Item = &'a GovernedSource>,
    rows: &[BoundaryRow],
) -> Vec<String> {
    failures_for_inputs(
        sources.map(|source| (source.relative.as_str(), source.bytes.as_slice())),
        rows,
    )
}

fn failures_for_inputs<'a>(
    sources: impl Iterator<Item = (&'a str, &'a [u8])>,
    rows: &[BoundaryRow],
) -> Vec<String> {
    let mut reports = BTreeMap::new();
    let mut failures = Vec::new();
    for (relative, bytes) in sources {
        match analyze(RustSyntaxRequest {
            source_path: relative,
            source_bytes: bytes,
            include_test_items: false,
        }) {
            Ok(report) => {
                reports.insert(relative.to_string(), report);
            }
            Err(error) => failures.push(error.stable_text()),
        }
    }
    failures.extend(registry::validation_failures(rows, &reports));
    for (path, report) in &reports {
        failures.extend(policy::authority_failures(path, report, rows));
    }
    failures.sort();
    failures.dedup();
    failures
}

pub(super) fn function<'a>(
    report: &'a RustSyntaxReport,
    symbol: &str,
) -> Vec<&'a crate::audit::source_governance::rust_syntax::FunctionShape> {
    report
        .functions
        .iter()
        .filter(|function| function.name == symbol)
        .collect()
}
