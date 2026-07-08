use super::changed_inputs::ChangedInputs;
use super::nodes::timing::NodeTiming;
use super::surfaces::{LOOP_VALIDATION_SURFACES, LoopValidationSurface};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::OnceLock;

mod claim_evaluation;
mod measurement_failure;
mod surface_record;
#[cfg(test)]
mod tests;
mod timing_projection_fields;
#[cfg(test)]
pub(crate) use surface_record::surface_record;

pub(crate) fn tasks(
    candidate_digest: &str,
    _changed_files_digest: &str,
    audit_context_digest: &str,
    tier: &str,
    cache_mode: &str,
    package_digest_baseline_ms: Option<u64>,
    node_timings: &BTreeMap<String, NodeTiming>,
    changed_inputs: &ChangedInputs,
) -> Vec<Box<dyn FnOnce() -> Value + Send + 'static>> {
    LOOP_VALIDATION_SURFACES
        .iter()
        .map(|surface| {
            let surface = *surface;
            let surface_changed_digest = changed_inputs.surface_digest(surface).to_string();
            let input_digest = surface_input_digest(
                surface,
                candidate_digest,
                &surface_changed_digest,
                audit_context_digest,
            );
            let baseline_ms = baseline_ms(surface.id, package_digest_baseline_ms);
            let node_timing = node_timings.get(surface.id).cloned();
            let tier = tier.to_string();
            let cache_mode = cache_mode.to_string();
            Box::new(move || {
                surface_record::surface_record(
                    surface,
                    &input_digest,
                    &tier,
                    &cache_mode,
                    baseline_ms,
                    node_timing,
                )
            }) as Box<dyn FnOnce() -> Value + Send + 'static>
        })
        .collect()
}

#[cfg(test)]
pub(crate) fn first_blocker(nodes: &[Value]) -> Option<Value> {
    first_product_blocker(nodes)
        .or_else(|| first_observability_blocker(nodes))
        .or_else(|| first_speed_blocker(nodes))
        .or_else(|| {
            nodes
                .iter()
                .find(|node| is_blocking_node(node))
                .map(blocker_record)
        })
}

pub(crate) fn first_product_blocker(nodes: &[Value]) -> Option<Value> {
    nodes
        .iter()
        .find(|node| is_product_blocking_node(node))
        .map(blocker_record)
}

pub(crate) fn first_observability_blocker(nodes: &[Value]) -> Option<Value> {
    nodes
        .iter()
        .find(|node| is_observability_blocking_node(node))
        .map(blocker_record)
}

pub(crate) fn first_speed_blocker(nodes: &[Value]) -> Option<Value> {
    nodes
        .iter()
        .find(|node| is_speed_blocking_node(node))
        .map(blocker_record)
}

fn blocker_record(node: &Value) -> Value {
    let id = node["node_id"].as_str().unwrap_or("unknown_loop_node");
    json!({
        "id": id,
        "surface": node["surface"].as_str().unwrap_or("loop_node"),
        "why_failed": node["why_failed"].as_str().unwrap_or("live-loop node failed"),
        "where_failed": node["where_failed"].as_str().unwrap_or("loop.run.node"),
        "failure_class": node["failure_class"].as_str().unwrap_or("live_loop_node_failure"),
        "next_repair": node["next_repair"].as_str().unwrap_or("repair the live-loop node, then rerun ultragoal loop run"),
        "narrow_rerun": node["narrow_rerun"].as_str().unwrap_or("target/debug/ultragoal --root . loop run --tier hot --cache-mode verified-local --jobs auto"),
        "broad_rerun": "source audit once after narrow observable proof passes",
        "claim_impact": node["claim_impact"].as_str().unwrap_or("source_local_live_loop_blocked")
    })
}

pub(crate) fn is_blocking_node(node: &Value) -> bool {
    matches!(
        node.get("status").and_then(Value::as_str),
        Some("blocked" | "fail" | "failed" | "partial")
    )
}

fn is_product_blocking_node(node: &Value) -> bool {
    is_blocking_node(node)
        && node
            .get("validation_status")
            .and_then(Value::as_str)
            .is_some_and(|status| status != "pass")
}

fn is_observability_blocking_node(node: &Value) -> bool {
    is_blocking_node(node)
        && node
            .get("validation_status")
            .and_then(Value::as_str)
            .is_some_and(|status| status == "pass")
        && node
            .get("observability_status")
            .and_then(Value::as_str)
            .is_some_and(|status| status != "pass")
}

fn is_speed_blocking_node(node: &Value) -> bool {
    is_blocking_node(node)
        && node
            .get("validation_status")
            .and_then(Value::as_str)
            .is_some_and(|status| status == "pass")
        && node
            .get("observability_status")
            .and_then(Value::as_str)
            .is_some_and(|status| status == "pass")
        && node
            .get("speed_claim_status")
            .and_then(Value::as_str)
            .is_some_and(|status| status != "supported")
}

#[cfg(test)]
pub(crate) fn required_high_frequency_validation_ids() -> Vec<&'static str> {
    LOOP_VALIDATION_SURFACES
        .iter()
        .filter(|surface| surface.high_frequency)
        .map(|surface| surface.id)
        .collect()
}

fn baseline_ms(id: &str, package_digest_baseline_ms: Option<u64>) -> Option<u64> {
    match id {
        "package_digest" => package_digest_baseline_ms,
        _ => None,
    }
}

pub(crate) fn surface_input_digest(
    surface: LoopValidationSurface,
    candidate_digest: &str,
    surface_changed_digest: &str,
    audit_context_digest: &str,
) -> String {
    let digest_material = match surface.id {
        "package_digest" => candidate_digest.to_string(),
        "changed_files" => surface_changed_digest.to_string(),
        "audit_context" => audit_context_digest.to_string(),
        _ if surface.high_frequency => format!(
            "affected-input={surface_changed_digest};context={audit_context_digest};surface={}",
            surface.id
        ),
        _ => format!(
            "candidate={candidate_digest};affected-input={surface_changed_digest};context={audit_context_digest};surface={}",
            surface.id
        ),
    };
    crate::digest::bytes(digest_material.as_bytes())
}

pub(crate) fn verified_local_cache_key(
    id: &str,
    digest: &str,
    tier: &str,
    cache_mode: &str,
) -> String {
    let validator_version = validator_version();
    crate::digest::bytes(
        format!(
            "surface={id};input={digest};validator={validator_version};law={};schema={};fixture={};tier={tier};cache={cache_mode};env=local",
            law_version(),
            schema_version(),
            fixture_version()
        )
        .as_bytes(),
    )
}

pub(crate) fn validator_version() -> String {
    static VERSION: OnceLock<String> = OnceLock::new();
    VERSION
        .get_or_init(|| crate::digest::bytes(validator_authority_material().as_bytes()))
        .clone()
}

pub(crate) fn law_version() -> &'static str {
    "observability-live-loop"
}

pub(crate) fn schema_version() -> &'static str {
    "harness-ultragoal.live-loop-node-timing.v1"
}

pub(crate) fn fixture_version() -> &'static str {
    "source-tree-current"
}

fn validator_authority_material() -> String {
    format!(
        "authority=ultragoal-cli-control-plane;cli=ultragoal;package_version={};os={};arch={};law={};schema={};fixture={}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        law_version(),
        schema_version(),
        fixture_version()
    )
}
