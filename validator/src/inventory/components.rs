use super::component_expectations::expected_names;
use super::fs::{
    PhysicalEntryDescriptor, check_symlink, component_name, physical_entry, regular_files, relative,
};
use super::registry::RegistryData;
use super::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use crate::context::ReadSession;
use std::path::Path;

struct InvalidComponentMetadata<'a> {
    reads: &'a ReadSession,
    root: &'a Path,
    path: &'a Path,
    stable_id: String,
    kind: &'a str,
    owner: &'a str,
    findings: &'a mut Vec<InventoryFinding>,
    entries: &'a mut Vec<InventoryEntry>,
}

fn record_invalid_metadata(request: InvalidComponentMetadata<'_>) -> Result<(), InventoryError> {
    let InvalidComponentMetadata {
        reads,
        root,
        path,
        stable_id,
        kind,
        owner,
        findings,
        entries,
    } = request;
    let relative = relative(root, path)?;
    findings.push(InventoryFinding::error(
        "invalid_component_metadata",
        Some(&stable_id),
        Some(&relative),
        "required name or description metadata is missing or invalid".to_owned(),
    ));
    entries.push(physical_entry(
        reads,
        root,
        path,
        PhysicalEntryDescriptor {
            stable_id,
            kind,
            owner,
            authority_state: AuthorityState::Context,
            active_status: ActiveStatus::ContextOnly,
            generator: None,
            provenance: Vec::new(),
            references: Vec::new(),
        },
    )?);
    Ok(())
}

pub(crate) fn discover_skills(
    reads: &ReadSession,
    root: &Path,
    registry: &RegistryData,
    entries: &mut Vec<InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
) -> Result<(), InventoryError> {
    let canonical = expected_names(registry, "SKILL:");
    for path in regular_files(reads, root, "skills")?
        .into_iter()
        .filter(|path| path.file_name().is_some_and(|name| name == "SKILL.md"))
    {
        let confined = check_symlink(root, &path, findings)?;
        let directory_name = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                InventoryError::InvalidRegistry(format!("invalid skill path {}", path.display()))
            })?
            .to_owned();
        if !confined {
            continue;
        }
        let Some(declared) = component_name(reads, &path, true) else {
            record_invalid_metadata(InvalidComponentMetadata {
                reads,
                root,
                path: &path,
                stable_id: format!("INVALID-SKILL:{directory_name}"),
                kind: "skill",
                owner: "OWN-PLUGIN-PRODUCT",
                findings,
                entries,
            })?;
            continue;
        };
        let stable_id = format!("SKILL:{declared}");
        let authority = if canonical.contains(&declared) {
            AuthorityState::Canonical
        } else {
            AuthorityState::Context
        };
        entries.push(physical_entry(
            reads,
            root,
            &path,
            PhysicalEntryDescriptor {
                stable_id,
                kind: "skill",
                owner: "OWN-PLUGIN-PRODUCT",
                authority_state: authority,
                active_status: ActiveStatus::Active,
                generator: None,
                provenance: Vec::new(),
                references: Vec::new(),
            },
        )?);
    }
    Ok(())
}

pub(crate) fn discover_agents(
    reads: &ReadSession,
    root: &Path,
    registry: &RegistryData,
    entries: &mut Vec<InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
) -> Result<(), InventoryError> {
    let required = expected_names(registry, "AGENT:");
    for path in regular_files(reads, root, ".codex/agents")? {
        let confined = check_symlink(root, &path, findings)?;
        let stem = path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                InventoryError::InvalidRegistry(format!("invalid agent path {}", path.display()))
            })?
            .to_owned();
        if !confined {
            continue;
        }
        let declared = crate::agent_manifest::inspect(reads, &path, &stem);
        let Some(declared) = declared.ok() else {
            findings.push(InventoryFinding::error(
                "invalid_agent_manifest",
                Some(&format!("INVALID-AGENT:{stem}")),
                Some(&relative(root, &path)?),
                "project agent manifest does not satisfy the current read-only schema".to_owned(),
            ));
            record_invalid_metadata(InvalidComponentMetadata {
                reads,
                root,
                path: &path,
                stable_id: format!("INVALID-AGENT:{stem}"),
                kind: "agent",
                owner: "OWN-ULTRA-ROOT",
                findings,
                entries,
            })?;
            continue;
        };
        entries.push(physical_entry(
            reads,
            root,
            &path,
            PhysicalEntryDescriptor {
                stable_id: format!("AGENT:{declared}"),
                kind: "agent",
                owner: "OWN-ULTRA-ROOT",
                authority_state: if required.contains(&declared) {
                    AuthorityState::Canonical
                } else {
                    AuthorityState::Context
                },
                active_status: ActiveStatus::Active,
                generator: None,
                provenance: Vec::new(),
                references: Vec::new(),
            },
        )?);
    }
    Ok(())
}
