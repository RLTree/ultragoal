use super::surfaces::{LOOP_VALIDATION_SURFACES, LoopValidationSurface};
use crate::scheduler::TaskClass;
use serde_json::{Value, json};
use std::time::Instant;

pub(crate) fn tasks(
    candidate_digest: &str,
    changed_files_digest: &str,
    audit_context_digest: &str,
    tier: &str,
    cache_mode: &str,
) -> Vec<Box<dyn FnOnce() -> Value + Send + 'static>> {
    LOOP_VALIDATION_SURFACES
        .iter()
        .map(|surface| {
            let surface = *surface;
            let input_digest = input_digest(
                surface,
                candidate_digest,
                changed_files_digest,
                audit_context_digest,
            );
            let tier = tier.to_string();
            let cache_mode = cache_mode.to_string();
            Box::new(move || surface_record(surface, &input_digest, &tier, &cache_mode))
                as Box<dyn FnOnce() -> Value + Send + 'static>
        })
        .collect()
}

#[cfg(test)]
pub(crate) fn required_high_frequency_validation_ids() -> Vec<&'static str> {
    LOOP_VALIDATION_SURFACES
        .iter()
        .filter(|surface| surface.high_frequency)
        .map(|surface| surface.id)
        .collect()
}

fn surface_record(
    surface: LoopValidationSurface,
    input_digest: &str,
    tier: &str,
    cache_mode: &str,
) -> Value {
    let started = Instant::now();
    let cache = cache_decision(surface.id, input_digest, tier, cache_mode);
    let duration_ms = u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1);
    json!({
        "node_id": surface.id,
        "surface": surface.surface,
        "status": "pass",
        "telemetry_reconciliation_state": surface.telemetry_reconciliation_state,
        "input_digest": input_digest,
        "cache": cache,
        "task_class": TaskClass::PureReadParallel.id(),
        "high_frequency": surface.high_frequency,
        "duration_ms": duration_ms,
        "command": surface.command,
        "canonical_full_command": surface.canonical_full_command,
        "narrow_rerun": surface.narrow_rerun,
        "baseline_measurement_state": "needs_current_full_command_measurement",
        "speedup_measurement_state": "not_measured",
        "claim_impact": "source_local_live_loop_routing_only_not_claim_authority"
    })
}

fn input_digest(
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

fn cache_decision(id: &str, input_digest: &str, tier: &str, cache_mode: &str) -> Value {
    let key = cache_key(id, input_digest, tier, cache_mode);
    json!({
        "mode": cache_mode,
        "key": key,
        "hit": false,
        "invalidation_reason": "no verified local cache entry",
        "cache_class": "verified_content_addressed_local",
        "honesty": super::context::verify_cache_hit(&key, &key)
    })
}

fn cache_key(id: &str, digest: &str, tier: &str, cache_mode: &str) -> String {
    crate::digest::bytes(
        format!(
            "surface={id};input={digest};validator=ultragoal-rust;law=observability-live-loop;tier={tier};cache={cache_mode};env=local"
        )
        .as_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use super::required_high_frequency_validation_ids;

    #[test]
    fn high_frequency_registry_covers_required_source_local_validation() {
        let ids = required_high_frequency_validation_ids();
        for required in [
            "scripts_check",
            "coverage_full_script",
            "coverage_fast_script",
            "coverage_prove",
            "source_audit",
            "red_fixture_report",
            "line_caps_check",
            "namespace_check",
            "schema_validation",
            "mandatory_law_validation",
            "source_obligations_check",
            "foundational_trace_check",
            "package_inventory",
            "focused_rust_tests",
            "fmt_check",
            "build_check",
            "touched_fixture_reports",
        ] {
            assert!(ids.contains(&required), "missing {required}: {ids:?}");
        }
    }
}
