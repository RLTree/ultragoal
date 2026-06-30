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

fn text(row: &Value, key: &str) -> String {
    row.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
