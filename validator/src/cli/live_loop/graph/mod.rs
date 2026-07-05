use super::nodes::timing::NodeTiming;
use super::surfaces::{LOOP_VALIDATION_SURFACES, LoopValidationSurface};
use serde_json::{Value, json};
use std::collections::BTreeMap;

mod measurement_failure;
mod node_record;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) use node_record::surface_record;

pub(crate) fn tasks(
    candidate_digest: &str,
    changed_files_digest: &str,
    audit_context_digest: &str,
    tier: &str,
    cache_mode: &str,
    package_digest_baseline_ms: Option<u64>,
    node_timings: &BTreeMap<String, NodeTiming>,
) -> Vec<Box<dyn FnOnce() -> Value + Send + 'static>> {
    LOOP_VALIDATION_SURFACES
        .iter()
        .map(|surface| {
            let surface = *surface;
            let input_digest = surface_input_digest(
                surface,
                candidate_digest,
                changed_files_digest,
                audit_context_digest,
            );
            let baseline_ms = baseline_ms(surface.id, package_digest_baseline_ms);
            let node_timing = node_timings.get(surface.id).cloned();
            let tier = tier.to_string();
            let cache_mode = cache_mode.to_string();
            Box::new(move || {
                node_record::surface_record(
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

pub(crate) fn first_blocker(nodes: &[Value]) -> Option<Value> {
    nodes.iter().find(|node| node["status"] != "pass").map(|node| {
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
    })
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
    changed_files_digest: &str,
    audit_context_digest: &str,
) -> String {
    let digest_material = match surface.id {
        "package_digest" => candidate_digest.to_string(),
        "changed_files" => changed_files_digest.to_string(),
        "audit_context" => audit_context_digest.to_string(),
        _ => format!(
            "candidate={candidate_digest};changed={changed_files_digest};context={audit_context_digest};surface={}",
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
    crate::digest::bytes(
        format!(
            "surface={id};input={digest};validator=ultragoal-rust;law=observability-live-loop;tier={tier};cache={cache_mode};env=local"
        )
        .as_bytes(),
    )
}
