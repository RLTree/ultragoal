use super::registry::{BoundaryRow, permits};
use crate::audit::source_governance::rust_syntax::RustSyntaxReport;

pub(super) fn authority_failures(
    path: &str,
    report: &RustSyntaxReport,
    rows: &[BoundaryRow],
) -> Vec<String> {
    report
        .authorities
        .iter()
        .filter_map(|authority| {
            let Some(symbol) = authority.function.as_deref() else {
                return Some(format!(
                    "raw_authority_outside_boundary:path={path};authority={};symbol=module",
                    authority.kind.id()
                ));
            };
            if permits(rows, path, symbol, authority.kind) {
                None
            } else {
                Some(format!(
                    "raw_authority_unregistered:path={path};symbol={symbol};authority={};repair=parse_once_into_closed_request_result_error_records",
                    authority.kind.id()
                ))
            }
        })
        .collect()
}
