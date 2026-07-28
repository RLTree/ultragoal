#[test]
fn raw_authority_scanner_allows_named_product_projection_boundaries() {
    for (rel, text) in [
        (
            "validator/src/cli/final_packet/proof/spans.rs",
            "use serde_json::{json, Value};\npub(crate) fn project(value: &Value) -> Value { let _ = span_kind(); let _ = receipt_deref(); let _ = dereferenced_receipt_digest(); json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/live_loop/context.rs",
            "use serde_json::{json, Value};\nstruct AuditContext;\npub(crate) fn project(value: &Value) -> Value { let _ = changed_files_digest(); let _ = input_digest(); json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/live_loop/graph/mod.rs",
            "use serde_json::{json, Value};\nstruct LoopValidationSurface;\npub(crate) fn project(value: &Value) -> Value { let _ = input_digest(); let _ = claim_impact(); json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/live_loop/receipt.rs",
            "use serde_json::{json, Value};\nuse std::path::Path;\nstruct AuditContext;\nstruct CommandTelemetry;\npub(crate) fn project(root: &Path, value: &Value) -> Value { let _ = root; let _ = \"live_loop_hot_repair_feedback\"; json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/observe/telemetry/claims.rs",
            "use serde_json::{json, Value};\nstruct ObserveOperation;\npub(crate) fn project(value: &Value) -> Value { let _ = \"observability_product_closure_failed_completion_readiness_release_update_goal_blocked\"; let _ = bounds_status(); json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/observe/telemetry/metric/mod.rs",
            "use serde_json::{json, Value};\npub(crate) fn project(value: &Value) -> Value { let _ = \"ultragoal_command_total\"; let _ = \"ultragoal_command_duration_ms\"; let _ = \"saturation_status\"; json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/observe/telemetry/trace/mod.rs",
            "use serde_json::{json, Value};\npub(crate) fn project(value: &Value) -> Value { let _ = child_spans(); let _ = parent_span_id(); let _ = span_kind(); json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/product/cohesion.rs",
            "use serde_json::{json, Value};\npub(crate) fn project(value: &Value) -> Value { let _ = \"product-cohesion\"; let _ = \"source_local_product_cohesion_only\"; json!({\"value\":value}) }\n",
        ),
    ] {
        let failures =
            crate::audit::law::authority_surfaces::raw_authority_failures_for_test(rel, text);
        assert!(failures.is_empty(), "{rel}: {failures:?}");
    }
}

#[test]
fn raw_authority_scanner_allows_typed_record_projection_value_aliases() {
    for (label, text) in [
        (
            "value_alias",
            "use serde_json as json;\ntype Value = json::Value;\npub struct ClaimState { pub status: String }\npub fn project(value: &Value) -> ClaimState { ClaimState { status: value.to_string() } }\n",
        ),
        (
            "value_collection_alias",
            "use serde_json as json;\ntype Value = json::Value;\npub fn project(values: &[Value]) -> Vec<String> { values.iter().map(|value| value.to_string()).collect() }\n",
        ),
    ] {
        let failures = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            "validator/src/audit/domain/claim_projection.rs",
            text,
        );
        assert!(
            failures.is_empty(),
            "{label} should classify as typed record projection: {failures:?}"
        );
    }
}
