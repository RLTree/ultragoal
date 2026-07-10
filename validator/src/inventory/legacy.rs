use super::fs::{check_symlink, physical_entry, read_bounded, relative};
use super::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use crate::context::ReadSession;
use std::fs;
use std::path::{Component, Path};

const MAX_LEGACY_TEXT_BYTES: u64 = 2 * 1024 * 1024;

enum ModelReference {
    Match,
    NoMatch,
    Rejected,
}

fn component(rel: &Path, wanted: &str) -> bool {
    rel.components()
        .any(|part| matches!(part, Component::Normal(name) if name == wanted))
}

fn primary_collection(lower: &str) -> bool {
    lower.starts_with("skills/")
        || lower.starts_with("schemas/")
        || lower.starts_with("fixtures/")
        || lower.starts_with(".codex/agents/")
        || lower.starts_with("generated/")
        || lower.starts_with("docs/generated/")
        || lower.starts_with("examples/generated/")
}

fn legacy_kind(rel: &Path) -> Option<&'static str> {
    let text = rel.to_string_lossy();
    let lower = text.to_ascii_lowercase();
    if primary_collection(&lower) {
        return None;
    }
    let file = rel
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if lower.starts_with("docs/ultragoal-contract-2026-07-successor-candidate-v1/")
        || lower.starts_with("docs/ultragoal-contract-2026-07/")
    {
        Some("contract")
    } else if file == "plugin-manifest-draft.json" {
        Some("manifest-projection")
    } else if component(rel, "custom-agents")
        || (component(rel, "agents") && !lower.starts_with(".codex/agents/"))
    {
        Some("agent")
    } else if file == "LANE_REGISTRY.json"
        || (!lower.starts_with("fixtures/") && component(rel, "lane"))
    {
        Some("lane")
    } else if !lower.starts_with("fixtures/")
        && (component(rel, "gate") || lower.contains("gate_registry"))
    {
        Some("gate")
    } else if lower.contains("command-inventory")
        || lower.contains("command_catalog")
        || lower.starts_with("validator/src/argument_parser/")
        || lower.starts_with("validator/src/command/")
    {
        Some("command")
    } else if !lower.starts_with("fixtures/")
        && (lower.contains("finalizer")
            || lower.contains("finalization")
            || lower.contains("final-packet")
            || lower.contains("final_packet"))
    {
        Some("finalizer")
    } else {
        None
    }
}

fn model_reference(
    reads: &ReadSession,
    path: &Path,
    rel: &Path,
) -> Result<ModelReference, InventoryError> {
    let text = rel.to_string_lossy();
    let lower = text.to_ascii_lowercase();
    if primary_collection(&lower)
        || lower.starts_with("docs/ultragoal-contract-2026-07-successor-v2/")
    {
        return Ok(ModelReference::NoMatch);
    }
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
        ModelReference::Match
    } else {
        ModelReference::NoMatch
    })
}

pub(crate) fn discover(
    reads: &ReadSession,
    root: &Path,
) -> Result<(Vec<InventoryEntry>, Vec<InventoryFinding>), InventoryError> {
    let paths = super::walk::repository_files(reads, root)?;
    let mut entries = Vec::new();
    let mut findings = Vec::new();
    for path in paths {
        let rel_text = relative(root, &path)?;
        let rel = Path::new(&rel_text);
        let kind = if let Some(kind) = legacy_kind(rel) {
            Some(kind)
        } else {
            match model_reference(reads, &path, rel)? {
                ModelReference::Match => Some("model"),
                ModelReference::NoMatch => None,
                ModelReference::Rejected => {
                    findings.push(InventoryFinding::error(
                        "legacy_model_scan_rejected",
                        None,
                        Some(&rel_text),
                        "regular file could not be classified within the bounded legacy model scan"
                            .to_owned(),
                    ));
                    None
                }
            }
        };
        let Some(kind) = kind else { continue };
        check_symlink(root, &path, &mut findings)?;
        entries.push(physical_entry(
            reads,
            root,
            &path,
            format!("LEGACY-{}:{rel_text}", kind.to_ascii_uppercase()),
            &format!("legacy-{kind}-authority"),
            "OWN-MAINTENANCE",
            AuthorityState::Legacy,
            ActiveStatus::Active,
            None,
            Vec::new(),
            Vec::new(),
        )?);
    }
    Ok((entries, findings))
}
