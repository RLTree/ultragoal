#[test]
fn raw_authority_scanner_allows_named_product_projection_boundaries() {
    for (rel, text) in [
        (
            "validator/src/cli/control/plane/mod.rs",
            "use serde_json::{json, Value};\nstruct ControlOperation;\npub(crate) fn project(value: &Value) -> Value { let _ = receipt_from_control_graph(); let _ = registry::stdout::print(); json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/control/plane/proof/mod.rs",
            "use serde_json::{json, Value};\nstruct ControlOperation;\npub(crate) fn project(value: &Value) -> Value { let _ = diagnostic::failure_value(); let _ = diagnostic::notes(); json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/control/plane/registry/capability/gap.rs",
            "use serde_json::{json, Value};\npub(crate) fn project(value: &Value) -> Value { let _ = missing_capability_class(); let _ = affected_claim_ids(); let _ = current_claim_ceiling(); json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/audit/receipt/scheduler_execution.rs",
            "use serde_json::Value;\nstruct Metrics;\npub(crate) fn project(metric: &Metrics, target_digest: &str) -> Value { let _ = \"crate::scheduler::Metrics\"; metric.to_value(target_digest, \"supports_source_local_scheduler_timing_only_not_readiness\") }\n",
        ),
        (
            "validator/src/audit/receipt/speed.rs",
            "use serde_json::{json, Value};\npub(crate) fn project(value: &Value) -> Value { let _ = target_ms(); let _ = hard_ceiling_ms(); let _ = source_local_speed_budget_only_not_readiness(); json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/control/plane/emit.rs",
            "use serde_json::{json, Value};\nuse std::path::Path;\nstruct ControlOperation;\npub(crate) fn project(root: &Path, value: &Value) -> Value { let _ = root; let _ = evidence::same_candidate_pass_failures(); let _ = required_evidence(); json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/control/plane/proof/diagnostic.rs",
            "use serde_json::{json, Value};\npub(crate) fn project(value: &Value) -> Value { let _ = claim_ceiling_impact(); let _ = source_install_cache_impact(); let _ = rerun_command(); json!({\"value\":value}) }\n",
        ),
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
            "use serde_json::{json, Value};\nuse std::path::Path;\nstruct AuditContext;\nstruct CommandTelemetry;\npub(crate) fn project(root: &Path, value: &Value) -> Value { let _ = root; let _ = \"observability_live_loop_source_local_increment\"; json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/observe/telemetry/claims.rs",
            "use serde_json::{json, Value};\nstruct ObserveOperation;\npub(crate) fn project(value: &Value) -> Value { let _ = \"observability_product_closure_failed_completion_readiness_release_update_goal_blocked\"; let _ = bounds_status(); json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/observe/telemetry/metric.rs",
            "use serde_json::{json, Value};\npub(crate) fn project(value: &Value) -> Value { let _ = \"ultragoal_command_total\"; let _ = \"ultragoal_command_duration_ms\"; let _ = \"saturation_status\"; json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/observe/telemetry/trace.rs",
            "use serde_json::{json, Value};\npub(crate) fn project(value: &Value) -> Value { let _ = child_spans(); let _ = parent_span_id(); let _ = span_kind(); json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/openai/config.rs",
            "use serde_json::{json, Value};\npub(crate) fn project(value: &Value) -> Value { let _ = openai_config_redacted_resolution(); let _ = secret_material_serialized(); let _ = blocked_claims(); json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/cli/product/cohesion.rs",
            "use serde_json::{json, Value};\npub(crate) fn project(value: &Value) -> Value { let _ = \"product-cohesion\"; let _ = \"source_local_product_cohesion_only\"; let _ = \"target_repo::product::cohesion::check\"; json!({\"value\":value}) }\n",
        ),
        (
            "validator/src/target_repo/receipt.rs",
            "use serde_json::{json, Value};\nstruct TargetReceiptInput;\npub(crate) fn project(value: &Value) -> Value { let _ = canonical_fingerprint(); let _ = repo_fingerprint(); json!({\"value\":value}) }\n",
        ),
    ] {
        let failures =
            crate::audit::law::authority_surfaces::raw_authority_failures_for_test(rel, text);
        assert!(failures.is_empty(), "{rel}: {failures:?}");
    }
}
