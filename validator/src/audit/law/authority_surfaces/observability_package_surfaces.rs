use super::RequiredSurface;

pub(super) const REQUIRED_SURFACES: &[RequiredSurface] = &[
    source("validator/src/cli/live_loop/nodes/measurement/cache_replay/telemetry_reuse.rs"),
    source("validator/src/cli/live_loop/nodes/measurement/observation/availability_query_tests.rs"),
    source(
        "validator/src/cli/live_loop/nodes/measurement/observation/artifact_publication_tests.rs",
    ),
    source("validator/src/cli/live_loop/nodes/measurement/observation/observe_receipt_reader.rs"),
    source("validator/src/cli/live_loop/nodes/measurement/observation/reconciliation_report.rs"),
    source("validator/src/cli/live_loop/nodes/measurement/observation/roundtrip_failure_tests.rs"),
    source(
        "validator/src/cli/live_loop/nodes/measurement/observation/successful_roundtrip_tests.rs",
    ),
    source("validator/src/cli/live_loop/nodes/measurement/observation/test_reconciliation.rs"),
    source("validator/src/cli/live_loop/nodes/measurement/query/mod.rs"),
    source("validator/src/cli/live_loop/nodes/measurement/query/receipt_capture.rs"),
    source("validator/src/cli/live_loop/nodes/measurement/query/reconciliation.rs"),
    source("validator/src/cli/live_loop/nodes/measurement/surface_selection.rs"),
    source("validator/src/cli/live_loop/nodes/measurement/cache/failure_tests.rs"),
    source("validator/src/cli/live_loop/nodes/measurement/cache/mod.rs"),
    source("validator/src/cli/live_loop/node_timing/tests/mod.rs"),
    source("validator/src/cli/live_loop/node_timing/tests/cache_replay.rs"),
    source("validator/src/cli/live_loop/node_timing/tests/receipt_writes.rs"),
    source("validator/src/cli/live_loop/node_timing/tests/stdout_fields.rs"),
    source("validator/src/cli/live_loop/nodes/timing/node_timing.rs"),
    source("validator/src/cli/observe/explain/query/evidence.rs"),
    source("validator/src/cli/observe/explain/query/evidence_tests.rs"),
    source("validator/src/cli/observe/explain/query/mod.rs"),
    source("validator/src/cli/observe/explain/query/receipt_target.rs"),
    source("validator/src/cli/observe/explain/receipt/catalog.rs"),
    source("validator/src/cli/observe/explain/receipt/event_target.rs"),
    source("validator/src/cli/observe/explain/receipt/mod.rs"),
    source("validator/src/cli/observe/explain/summary_tests.rs"),
    source("validator/src/cli/observe/explain/tests/observation/query_receipts.rs"),
    source("validator/src/cli/observe/explain/tests/observation/skip.rs"),
    source("validator/src/cli/observe/explain/tests/observation/staleness.rs"),
    source("validator/src/cli/observe/explain/tests/telemetry_receipt.rs"),
    source("validator/src/cli/observe/query/records_tests.rs"),
    source("validator/src/cli/observe/query/retry.rs"),
    source("validator/src/cli/observe/query/trace_transport.rs"),
    source("validator/src/cli/observe/telemetry/inventory_status.rs"),
    source("validator/src/cli/observe/telemetry/metric/export.rs"),
    source("validator/src/cli/observe/telemetry/metric/mod.rs"),
    source("validator/src/cli/observe/telemetry/metric/summary_tests.rs"),
    source("validator/src/cli/observe/telemetry/stack_receipt.rs"),
    source("validator/src/cli/observe/telemetry/trace/export.rs"),
    source("validator/src/cli/observe/telemetry/trace/mod.rs"),
];

const fn source(rel: &'static str) -> RequiredSurface {
    RequiredSurface {
        role: "source",
        rel,
        package_inventory_required: true,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn observability_package_surface_rows_are_source_inventory_requirements() {
        let row = super::source("validator/src/cli/observe/query/trace_transport.rs");

        assert_eq!(row.role, "source");
        assert_eq!(
            row.rel,
            "validator/src/cli/observe/query/trace_transport.rs"
        );
        assert!(row.package_inventory_required);
        assert!(
            super::REQUIRED_SURFACES
                .iter()
                .any(|surface| surface.rel == "validator/src/cli/observe/query/trace_transport.rs")
        );
    }
}
