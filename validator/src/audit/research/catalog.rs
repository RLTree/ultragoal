use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const REQUIRED_SOURCES: &[&str] = &[
    "openai-harness-engineering",
    "openai-codex-iterative-repair-loops",
    "openai-agents-observability",
    "openai-agents-sdk-tracing",
    "google-sre-monitoring-workbook",
    "google-sre-four-golden-signals",
    "charity-majors-structured-events",
    "honeycomb-high-cardinality",
    "opentelemetry-semantic-conventions",
    "openai-agent-improvement-loop",
    "openai-self-improving-tax-agent",
    "attached-rust-devx-guide",
    "attached-typescript-frontend-guide",
];

pub(super) fn source_cards(cards: &Value) -> BTreeSet<String> {
    cards
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("source_id").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

pub(super) fn registry_sources(registry: &Value) -> BTreeMap<String, Value> {
    registry
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            Some((text(row, "source_id"), row.clone())).filter(|(id, _)| !id.is_empty())
        })
        .collect()
}

pub(super) fn card_requirements(cards: &Value) -> BTreeMap<String, String> {
    cards
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|source| {
            let source_id = text(source, "source_id");
            source
                .get("requirements")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(move |requirement| {
                    Some((text(requirement, "requirement_id"), source_id.clone()))
                        .filter(|(id, source_id)| !id.is_empty() && !source_id.is_empty())
                })
        })
        .collect()
}

pub(super) fn source_artifacts(cards: &Value) -> BTreeMap<String, (String, String)> {
    cards
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let id = text(row, "source_id");
            let digest = text(row, "source_artifact_digest");
            let method = text(row, "source_artifact_method");
            (!id.is_empty()).then_some((id, (digest, method)))
        })
        .collect()
}

pub(super) fn source_evidence_failures(cards: &Value) -> Vec<String> {
    cards
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(source_evidence_failures_for)
        .collect()
}

pub(super) fn trace_entries(trace: &Value) -> BTreeMap<String, Value> {
    trace
        .get("entries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            Some((text(row, "requirement_id"), row.clone())).filter(|(id, _)| !id.is_empty())
        })
        .collect()
}

pub(super) fn required_source_failures(registry: &BTreeMap<String, Value>) -> Vec<String> {
    REQUIRED_SOURCES
        .iter()
        .filter(|id| !registry.contains_key(**id))
        .map(|id| format!("research_registry_missing_required_source:{id}"))
        .collect()
}

fn source_evidence_failures_for(source: &Value) -> Vec<String> {
    let id = text(source, "source_id");
    let mut out = Vec::new();
    let evidence_ids = evidence_ids(source);
    if text(source, "source_artifact_digest").is_empty() {
        out.push(format!("research_source_card_artifact_digest_missing:{id}"));
    }
    if text(source, "source_artifact_method").is_empty() {
        out.push(format!("research_source_card_artifact_method_missing:{id}"));
    }
    if evidence_ids.is_empty() {
        out.push(format!("research_source_card_evidence_missing:{id}"));
    }
    source
        .get("requirements")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .for_each(|requirement| {
            let requirement_id = text(requirement, "requirement_id");
            let anchors = array(requirement, "source_evidence_ids");
            if anchors.is_empty() {
                out.push(format!(
                    "research_source_card_requirement_unanchored:{requirement_id}"
                ));
            }
            for anchor in anchors {
                if !evidence_ids.contains(&anchor) {
                    out.push(format!(
                        "research_source_card_requirement_unknown_anchor:{requirement_id}:{anchor}"
                    ));
                }
            }
        });
    out
}

fn evidence_ids(source: &Value) -> BTreeSet<String> {
    source
        .get("evidence_anchors")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("evidence_id").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

fn array(row: &Value, key: &str) -> Vec<String> {
    row.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn text(row: &Value, key: &str) -> String {
    row.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
