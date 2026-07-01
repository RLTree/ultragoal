use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

mod artifacts;

pub(super) use artifacts::{SourceArtifact, source_artifacts};

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
    "agentic-gold-standard-stack-synthesis-2026-07-01",
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
    let canonical_url = text(source, "canonical_url");
    let requirement_ids = requirement_ids(source);
    let evidence_ids = evidence_ids(source);
    if id.is_empty() {
        out.push("research_source_card_id_missing".to_string());
    }
    if canonical_url.is_empty() {
        out.push(format!("research_source_card_canonical_url_missing:{id}"));
    }
    if text(source, "source_kind").is_empty() {
        out.push(format!("research_source_card_kind_missing:{id}"));
    }
    if text(source, "source_artifact_digest").is_empty() {
        out.push(format!("research_source_card_artifact_digest_missing:{id}"));
    }
    if text(source, "source_artifact_method").is_empty() {
        out.push(format!("research_source_card_artifact_method_missing:{id}"));
    }
    if text(source, "source_corpus_path").is_empty() {
        out.push(format!("research_source_card_corpus_path_missing:{id}"));
    }
    if text(source, "source_corpus_digest").is_empty() {
        out.push(format!("research_source_card_corpus_digest_missing:{id}"));
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
            if requirement_id.is_empty() {
                out.push(format!("research_source_card_requirement_id_missing:{id}"));
            }
            if text(requirement, "summary").is_empty() {
                out.push(format!(
                    "research_source_card_requirement_summary_missing:{requirement_id}"
                ));
            }
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
    source
        .get("evidence_anchors")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .for_each(|anchor| {
            let evidence_id = text(anchor, "evidence_id");
            if evidence_id.is_empty() {
                out.push(format!("research_source_card_evidence_id_missing:{id}"));
            }
            let locator = text(anchor, "source_locator");
            if locator.is_empty() {
                out.push(format!(
                    "research_source_card_evidence_locator_missing:{id}:{evidence_id}"
                ));
            } else if !canonical_url.is_empty() && !locator.starts_with(&canonical_url) {
                out.push(format!(
                    "research_source_card_evidence_locator_mismatch:{id}:{evidence_id}"
                ));
            }
            if text(anchor, "source_signal").is_empty() {
                out.push(format!(
                    "research_source_card_evidence_signal_missing:{id}:{evidence_id}"
                ));
            }
            let anchor_requirements = array(anchor, "requirement_ids");
            if anchor_requirements.is_empty() {
                out.push(format!(
                    "research_source_card_evidence_requirement_missing:{id}:{evidence_id}"
                ));
            }
            for requirement_id in anchor_requirements {
                if !requirement_ids.contains(&requirement_id) {
                    out.push(format!(
                        "research_source_card_evidence_unknown_requirement:{id}:{evidence_id}:{requirement_id}"
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

fn requirement_ids(source: &Value) -> BTreeSet<String> {
    source
        .get("requirements")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("requirement_id").and_then(Value::as_str))
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
