use super::DraftPackageManifest;
use std::collections::BTreeSet;

const MAX_ROWS: usize = 100_000;
const MAX_AGGREGATE_TEXT_BYTES: usize = 64 * 1024 * 1024;

pub(super) fn validate_manifest(manifest: &DraftPackageManifest) -> Result<(), String> {
    if manifest.skills.len() > MAX_ROWS
        || manifest.agents.len() > MAX_ROWS
        || manifest.optional_connectors.len() > MAX_ROWS
        || manifest.non_goals.len() > MAX_ROWS
    {
        return Err("package manifest exceeds its row limit".to_owned());
    }
    match manifest.status {
        super::field_types::DraftStatus::Implementation | super::field_types::DraftStatus::Test => {
        }
    }
    let semantic_text_bytes = manifest.purpose.as_str().len()
        + manifest
            .skills
            .iter()
            .map(|row| row.role.as_str().len())
            .sum::<usize>()
        + manifest
            .non_goals
            .iter()
            .map(|row| row.as_str().len())
            .sum::<usize>();
    if semantic_text_bytes > MAX_AGGREGATE_TEXT_BYTES {
        return Err("package manifest exceeds its semantic text limit".to_owned());
    }
    reject_duplicate_semantic_ids(
        manifest.skills.iter().map(|row| row.name.as_str()),
        "skill identifier",
    )?;
    reject_duplicate_semantic_ids(
        manifest.agents.iter().map(|row| row.name.as_str()),
        "agent identifier",
    )?;
    reject_duplicate_semantic_ids(
        manifest
            .optional_connectors
            .iter()
            .map(|connector| connector.as_str()),
        "connector identifier",
    )?;
    reject_path_collisions(manifest.inventory_paths())
}

fn reject_duplicate_semantic_ids<'a>(
    values: impl IntoIterator<Item = &'a str>,
    label: &str,
) -> Result<(), String> {
    let mut folded = BTreeSet::new();
    for value in values {
        if !folded.insert(value.to_ascii_lowercase()) {
            return Err(format!("package manifest contains a duplicate {label}"));
        }
    }
    Ok(())
}

fn reject_path_collisions(paths: Vec<String>) -> Result<(), String> {
    if paths.len() > MAX_ROWS {
        return Err("package manifest exceeds its path limit".to_owned());
    }
    let mut exact_paths = BTreeSet::new();
    let mut folded_paths = BTreeSet::new();
    for path in paths {
        if !exact_paths.insert(path.clone()) {
            return Err("package manifest contains a duplicate path".to_owned());
        }
        if !folded_paths.insert(path.to_ascii_lowercase()) {
            return Err("package manifest contains a case-colliding path".to_owned());
        }
    }
    Ok(())
}
