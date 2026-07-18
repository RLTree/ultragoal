use super::super::fs::read_bounded;
use super::super::types::InventoryError;
use crate::context::ReadSession;
use std::fs;
use std::path::{Component, Path};

const MAX_LEGACY_TEXT_BYTES: u64 = 2 * 1024 * 1024;

pub(super) struct LegacyMatch {
    pub(super) kind: &'static str,
    pub(super) evidence: &'static str,
}

pub(super) enum ModelReference {
    Match(LegacyMatch),
    NoMatch,
    Rejected,
}

fn found(kind: &'static str, evidence: &'static str) -> Option<LegacyMatch> {
    Some(LegacyMatch { kind, evidence })
}

fn component(rel: &Path, wanted: &str) -> bool {
    rel.components()
        .any(|part| matches!(part, Component::Normal(name) if name == wanted))
}

pub(super) fn path_match(rel: &Path) -> Option<LegacyMatch> {
    let text = rel.to_string_lossy();
    let lower = text.to_ascii_lowercase();
    let file = rel
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if lower.starts_with("docs/ultragoal-contract-2026-07/") {
        found("contract", "path:adopted-predecessor-contract")
    } else if lower.starts_with("docs/ultragoal-contract-2026-07-successor-candidate-v1/") {
        found("contract", "path:unverified-successor-candidate")
    } else if file == "REPORT.md"
        && rel
            .parent()
            .is_some_and(|parent| parent.as_os_str().is_empty())
    {
        found("proposal", "path:unrouted-predecessor-proposal")
    } else if file == "plugin-manifest-draft.json" {
        found("manifest-projection", "path:legacy-plugin-manifest-draft")
    } else if component(rel, "custom-agents")
        || (component(rel, "agents") && !lower.starts_with(".codex/agents/"))
    {
        found("agent", "path:legacy-agent-collection")
    } else if matches!(
        text.as_ref(),
        "validator/src/claim_semantics/ready/mod.rs"
            | "validator/src/claim_semantics/ready/receipt.rs"
    ) {
        found("finalizer", "path:retired-ready-authority")
    } else if file == "LANE_REGISTRY.json" || component(rel, "lane") {
        found("lane", "path:lane-authority")
    } else if component(rel, "gate") || lower.contains("gate_registry") {
        found("gate", "path:gate-authority")
    } else if lower.contains("command-inventory")
        || file == "command-catalog.json"
        || lower.contains("command_catalog")
        || lower.starts_with("validator/src/argument_parser/")
        || lower.starts_with("validator/src/command/")
    {
        found("command", "path:legacy-command-authority")
    } else if component(rel, "final_packet")
        || lower.contains("finalizer")
        || lower.contains("finalization")
        || lower.contains("final-packet")
    {
        found("finalizer", "path:legacy-finalization-authority")
    } else {
        None
    }
}

pub(super) fn model_reference(
    reads: &ReadSession,
    path: &Path,
    _rel: &Path,
) -> Result<ModelReference, InventoryError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| InventoryError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Ok(ModelReference::NoMatch);
    }
    let bytes = match read_bounded(reads, path, MAX_LEGACY_TEXT_BYTES) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(ModelReference::Rejected),
    };
    let Ok(content) = std::str::from_utf8(&bytes) else {
        return Ok(ModelReference::NoMatch);
    };
    Ok(if content.contains("gpt-5.5") {
        ModelReference::Match(LegacyMatch {
            kind: "model",
            evidence: "content:fixed-model-reference",
        })
    } else {
        ModelReference::NoMatch
    })
}
