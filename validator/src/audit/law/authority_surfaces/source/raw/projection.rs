pub(super) fn boundary_text(rel: &str, text: &str) -> bool {
    typed_record_projection_text(text) || classified_product_projection_boundary(rel, text)
}

fn classified_product_projection_boundary(rel: &str, text: &str) -> bool {
    let required: &[&str] = match rel {
        "validator/src/cli/control/plane/mod.rs" => &[
            "ControlOperation",
            "receipt_from_control_graph",
            "registry::stdout::print",
        ],
        "validator/src/cli/control/plane/proof/mod.rs" => &[
            "ControlOperation",
            "diagnostic::failure_value",
            "diagnostic::notes",
        ],
        "validator/src/cli/control/plane/registry/capability/gap.rs" => &[
            "missing_capability_class",
            "affected_claim_ids",
            "current_claim_ceiling",
        ],
        "validator/src/audit/receipt/scheduler_execution.rs" => &[
            "crate::scheduler::Metrics",
            "metric.to_value",
            "supports_source_local_scheduler_timing_only_not_readiness",
        ],
        "validator/src/audit/receipt/speed.rs" => &[
            "target_ms",
            "hard_ceiling_ms",
            "source_local_speed_budget_only_not_readiness",
        ],
        "validator/src/cli/control/plane/emit.rs" => &[
            "ControlOperation",
            "evidence::same_candidate_pass_failures",
            "required_evidence",
        ],
        "validator/src/cli/control/plane/proof/diagnostic.rs" => &[
            "claim_ceiling_impact",
            "source_install_cache_impact",
            "rerun_command",
        ],
        "validator/src/cli/final_packet/proof/spans.rs" => {
            &["span_kind", "receipt_deref", "dereferenced_receipt_digest"]
        }
        "validator/src/cli/live_loop/context.rs" => {
            &["AuditContext", "changed_files_digest", "input_digest"]
        }
        "validator/src/cli/live_loop/graph/mod.rs" => {
            &["LoopValidationSurface", "input_digest", "claim_impact"]
        }
        "validator/src/cli/live_loop/receipt.rs" => &[
            "AuditContext",
            "CommandTelemetry",
            "observability_live_loop_source_local_increment",
        ],
        "validator/src/cli/observe/explain/summary.rs" => {
            &["ExplainContext", "smallest_repair", "query_evidence"]
        }
        "validator/src/cli/observe/telemetry/claims.rs" => &[
            "ObserveOperation",
            "observability_product_closure_failed_completion_readiness_release_update_goal_blocked",
            "bounds_status",
        ],
        "validator/src/cli/observe/telemetry/metric.rs" => &[
            "ultragoal_command_total",
            "ultragoal_command_duration_ms",
            "saturation_status",
        ],
        "validator/src/cli/observe/telemetry/trace.rs" => {
            &["child_spans", "parent_span_id", "span_kind"]
        }
        "validator/src/cli/openai/config.rs" => &[
            "openai_config_redacted_resolution",
            "secret_material_serialized",
            "blocked_claims",
        ],
        "validator/src/audit/law/authority_surfaces/surface_inventory/mod.rs" => &[
            "AuthoritySurfaceInventoryRow",
            "harness-ultragoal.foundational-law-surface-inventory.v1",
            "surface_state",
        ],
        "validator/src/cli/product/cohesion.rs" => &[
            "product-cohesion",
            "source_local_product_cohesion_only",
            "target_repo::product::cohesion::check",
        ],
        "validator/src/target_repo/receipt.rs" => &[
            "TargetReceiptInput",
            "canonical_fingerprint",
            "repo_fingerprint",
        ],
        _ => return false,
    };
    required.iter().all(|needle| text.contains(needle)) && projection_value_text(text)
}

fn projection_value_text(text: &str) -> bool {
    text.contains("json!(")
        || text.contains("json!({")
        || text.contains("Value::Array(")
        || text.contains("Value::Object")
        || text.contains("serde_json::Map::new")
        || text.contains("serde_json::to_string")
        || text.contains("serde_json::to_value")
        || text.contains("canonical_json")
        || text.contains("stdout_contract")
        || text.contains("print_receipt")
        || text.contains("csv(")
        || text.contains("checks: &mut serde_json::Map")
        || text.contains("serde_json::Map<String, Value>")
        || text.contains("checks.insert(")
        || text.contains("metric.to_value(")
        || text.contains("FnOnce() -> Value")
        || text.contains("diagnostic::failure_value")
        || text.contains("receipt_from_control_graph")
}

fn typed_record_projection_text(text: &str) -> bool {
    if text.contains("-> Value")
        || text.contains("-> Result<Value")
        || text.contains("-> Option<Value")
        || text.contains("Value) -> Value")
    {
        return false;
    }
    let returns_typed_record = text.contains("-> String")
        || text.contains("-> &str")
        || text.contains("-> bool")
        || text.contains("-> Option<")
        || text.contains("-> Vec<")
        || (text.contains("-> BTreeMap<")
            && !text.contains("BTreeMap<String, Value")
            && !text.contains("HashMap<String, Value"))
        || text.contains("-> crate::cli::observe::telemetry::RuntimeTelemetry")
        || (text.contains("struct ") && text.contains("-> "));
    returns_typed_record
        && (super::contains_json_value_binding(text)
            || text.contains(": &Value")
            || text.contains(": &[Value]"))
}
