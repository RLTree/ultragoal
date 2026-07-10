use super::authority;
use super::metadata::{self, GeneratedMetadata};
use crate::context::ReadSession;
use crate::inventory::fs::{check_symlink, physical_entry, regular_files, relative};
use crate::inventory::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn discover(
    reads: &ReadSession,
    root: &Path,
    contract_id: &str,
    entries: &mut Vec<InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
) -> Result<(), InventoryError> {
    let (authority, authority_entry) = authority::load(reads, root, contract_id)?;
    entries.push(authority_entry);
    let mut seen = BTreeSet::new();
    for directory in ["generated", "docs/generated", "examples/generated"] {
        for path in regular_files(reads, root, directory)? {
            let confined = check_symlink(root, &path, findings)?;
            let rel = relative(root, &path)?;
            seen.insert(rel.clone());
            let metadata = if confined {
                metadata::inspect(reads, root, &path, authority.get(&rel))
            } else {
                GeneratedMetadata {
                    generator: None,
                    inputs: Vec::new(),
                    problems: Vec::new(),
                }
            };
            for (code, message) in &metadata.problems {
                findings.push(InventoryFinding::error(
                    code,
                    Some(&format!("GENERATED:{rel}")),
                    Some(&rel),
                    (*message).to_owned(),
                ));
            }
            if metadata.generator.is_none() {
                findings.push(InventoryFinding::warning(
                    "generated_surface_missing_provenance",
                    Some(&format!("GENERATED:{rel}")),
                    Some(&rel),
                    "generated surface has no generator metadata".to_owned(),
                ));
            }
            entries.push(physical_entry(
                reads,
                root,
                &path,
                format!("GENERATED:{rel}"),
                "generated-surface",
                "OWN-PRODUCT-ARCHITECTURE",
                AuthorityState::Projection,
                ActiveStatus::ContextOnly,
                metadata.generator.clone(),
                metadata.inputs,
                metadata.generator.into_iter().collect(),
            )?);
        }
    }
    for output in authority.keys().filter(|output| !seen.contains(*output)) {
        findings.push(InventoryFinding::error(
            "registered_generated_surface_missing",
            Some(&format!("GENERATED:{output}")),
            Some(output),
            "externally authorized generated output is missing".to_owned(),
        ));
    }
    Ok(())
}
