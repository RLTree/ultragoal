use crate::{digest, json_boundary};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod catalog;
mod trace;

const CARDS_PATH: &str = "docs/research-source-cards.json";
const REGISTRY_PATH: &str = "docs/research-source-registry.json";
const TRACE_PATH: &str = "docs/research-article-to-law-trace.json";
const GATE_92_LAW: &str = "full-local-observability-stack-integration-non-opaque-failure";

#[cfg(test)]
mod tests;

pub fn failures(root: &Path) -> Vec<String> {
    let cards = read(root, CARDS_PATH);
    let registry = read(root, REGISTRY_PATH);
    let trace = read(root, TRACE_PATH);
    match (cards, registry, trace) {
        (Ok(cards), Ok(registry), Ok(trace)) => value_failures(root, &cards, &registry, &trace),
        (cards, registry, trace) => [cards.err(), registry.err(), trace.err()]
            .into_iter()
            .flatten()
            .collect(),
    }
}

pub(crate) fn value_failures(
    root: &Path,
    cards: &Value,
    registry: &Value,
    trace: &Value,
) -> Vec<String> {
    let source_cards = catalog::source_cards(cards);
    let source_artifacts = catalog::source_artifacts(cards);
    let registry_sources = catalog::registry_sources(registry);
    let requirements = catalog::card_requirements(cards);
    let trace_entries = catalog::trace_entries(trace);
    let mut out = Vec::new();
    out.extend(catalog::source_evidence_failures(cards));
    out.extend(catalog::required_source_failures(&registry_sources));
    out.extend(registry_failures(
        root,
        cards,
        &source_cards,
        &source_artifacts,
        &registry_sources,
    ));
    out.extend(trace::coverage_failures(&requirements, &trace_entries));
    out.extend(trace::row_failures(root, &requirements, &trace_entries));
    out
}

fn read(root: &Path, rel: &str) -> Result<Value, String> {
    json_boundary::read_json(&root.join(rel)).map_err(|err| format!("{rel}: {err}"))
}

fn registry_failures(
    root: &Path,
    cards: &Value,
    source_cards: &BTreeSet<String>,
    source_artifacts: &BTreeMap<String, catalog::SourceArtifact>,
    registry: &BTreeMap<String, Value>,
) -> Vec<String> {
    let cards_digest = digest::file(&root.join(CARDS_PATH)).unwrap_or_default();
    let package_paths = package_inventory_paths(root);
    registry
        .iter()
        .flat_map(|(id, row)| {
            let mut out = Vec::new();
            if !source_cards.contains(id) {
                out.push(format!("research_registry_source_card_missing:{id}"));
            }
            if text(row, "source_card_path") != CARDS_PATH {
                out.push(format!("research_registry_source_card_path_invalid:{id}"));
            }
            if text(row, "source_card_digest") != cards_digest {
                out.push(format!("research_registry_source_card_digest_stale:{id}"));
            }
            match source_artifacts.get(id) {
                Some(artifact) => {
                    if text(row, "source_artifact_digest") != artifact.digest {
                        out.push(format!(
                            "research_registry_source_artifact_digest_stale:{id}"
                        ));
                    }
                    if text(row, "source_artifact_method") != artifact.method {
                        out.push(format!(
                            "research_registry_source_artifact_method_stale:{id}"
                        ));
                    }
                    if text(row, "source_corpus_path") != artifact.corpus_path {
                        out.push(format!("research_registry_source_corpus_path_stale:{id}"));
                    }
                    if text(row, "source_corpus_digest") != artifact.corpus_digest {
                        out.push(format!("research_registry_source_corpus_digest_stale:{id}"));
                    }
                    out.extend(source_corpus_failures(
                        root,
                        id,
                        &artifact.corpus_path,
                        &artifact.corpus_digest,
                        &package_paths,
                    ));
                }
                None => out.push(format!("research_registry_source_artifact_missing:{id}")),
            }
            if array(row, "canonical_law_ids_affected").is_empty() {
                out.push(format!("research_registry_unmapped_source:{id}"));
            }
            if !array(row, "canonical_law_ids_affected")
                .iter()
                .any(|law| law == GATE_92_LAW)
            {
                out.push(format!("research_registry_gate92_law_missing:{id}"));
            }
            if text(row, "claim_ceiling_impact").is_empty() {
                out.push(format!("research_registry_missing_claim_ceiling:{id}"));
            }
            if cards.get("schema").and_then(Value::as_str)
                != Some("harness-ultragoal.research-source-cards.v1")
            {
                out.push("research_source_cards_wrong_schema".to_string());
            }
            out
        })
        .collect()
}

fn package_inventory_paths(root: &Path) -> BTreeSet<String> {
    json_boundary::read_json(&root.join("plugin-manifest-draft.json"))
        .ok()
        .map(|manifest| crate::package::inventory::inventory_paths(&manifest))
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn source_corpus_failures(
    root: &Path,
    id: &str,
    rel: &str,
    expected_digest: &str,
    package_paths: &BTreeSet<String>,
) -> Vec<String> {
    let mut out = Vec::new();
    if rel.is_empty() {
        out.push(format!("research_source_corpus_path_missing:{id}"));
        return out;
    }
    if !rel.starts_with("artifacts/source-snapshots/") {
        out.push(format!(
            "research_source_corpus_path_not_snapshot:{id}:{rel}"
        ));
    }
    if !package_paths.contains(rel) {
        out.push(format!("research_source_corpus_package_omitted:{id}:{rel}"));
    }
    let path = match crate::package::inventory::resolve(root, rel) {
        Ok(path) => path,
        Err(err) => {
            out.push(format!("research_source_corpus_path_invalid:{id}:{err}"));
            return out;
        }
    };
    if !path.exists() {
        out.push(format!("research_source_corpus_missing:{id}:{rel}"));
        return out;
    }
    match digest::file(&path) {
        Ok(actual) if actual == expected_digest => {}
        Ok(_) => out.push(format!("research_source_corpus_digest_stale:{id}:{rel}")),
        Err(err) => out.push(format!("research_source_corpus_digest_error:{id}:{err}")),
    }
    out
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
