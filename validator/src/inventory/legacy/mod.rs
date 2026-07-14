mod matchers;
mod scope;

use self::matchers::{LegacyMatch, ModelReference};
use super::fs::{PhysicalEntryDescriptor, check_symlink, physical_entry, relative};
use super::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use crate::context::ReadSession;
use std::path::{Path, PathBuf};

fn model_match(
    reads: &ReadSession,
    path: &Path,
    rel: &Path,
    rel_text: &str,
    findings: &mut Vec<InventoryFinding>,
) -> Result<Option<LegacyMatch>, InventoryError> {
    Ok(match matchers::model_reference(reads, path, rel)? {
        ModelReference::Match(found) => Some(found),
        ModelReference::NoMatch => None,
        ModelReference::Rejected => {
            findings.push(InventoryFinding::error(
                "legacy_model_scan_rejected",
                None,
                Some(rel_text),
                "regular file could not be classified within the bounded legacy model scan"
                    .to_owned(),
            ));
            None
        }
    })
}

fn entry(
    reads: &ReadSession,
    root: &Path,
    path: &Path,
    rel_text: &str,
    found: LegacyMatch,
) -> Result<InventoryEntry, InventoryError> {
    physical_entry(
        reads,
        root,
        path,
        PhysicalEntryDescriptor {
            stable_id: format!("LEGACY-{}:{rel_text}", found.kind.to_ascii_uppercase()),
            kind: &format!("legacy-{}-authority", found.kind),
            owner: "OWN-MAINTENANCE",
            authority_state: AuthorityState::Legacy,
            active_status: ActiveStatus::Active,
            generator: None,
            provenance: Vec::new(),
            references: vec![format!("legacy-scope-evidence:{}", found.evidence)],
        },
    )
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
        if !check_symlink(root, &path, &mut findings)? {
            continue;
        }
        let rel = PathBuf::from(&rel_text);
        if !scope::inspect(&rel) {
            continue;
        }
        let found = if let Some(found) = matchers::path_match(&rel) {
            Some(found)
        } else {
            model_match(reads, &path, &rel, &rel_text, &mut findings)?
        };
        if let Some(found) = found {
            entries.push(entry(reads, root, &path, &rel_text, found)?);
        }
    }
    Ok((entries, findings))
}
