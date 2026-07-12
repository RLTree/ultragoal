use super::compatibility::{STRUCTURAL_WITNESS_KIND, by_legacy_name, inspect_wrapper};
use super::component_expectations::expected_names;
use super::fs::{check_symlink, component_name, physical_entry, regular_files, relative};
use super::registry::RegistryData;
use super::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use crate::context::ReadSession;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
fn record_invalid_metadata(
    reads: &ReadSession,
    root: &Path,
    path: &Path,
    stable_id: String,
    kind: &str,
    owner: &str,
    findings: &mut Vec<InventoryFinding>,
    entries: &mut Vec<InventoryEntry>,
) -> Result<(), InventoryError> {
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
        stable_id,
        kind,
        owner,
        AuthorityState::Context,
        ActiveStatus::ContextOnly,
        None,
        Vec::new(),
        Vec::new(),
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
            record_invalid_metadata(
                reads,
                root,
                &path,
                format!("INVALID-SKILL:{directory_name}"),
                "skill",
                "OWN-PLUGIN-PRODUCT",
                findings,
                entries,
            )?;
            continue;
        };
        let legacy = registry.legacy_skills.contains_key(&declared)
            || registry.legacy_skills.contains_key(&directory_name);
        let route_expected = legacy
            .then(|| by_legacy_name(&declared).or_else(|| by_legacy_name(&directory_name)))
            .flatten();
        let route_witness = if route_expected.is_some() {
            inspect_wrapper(reads, root, &path, &directory_name, &declared)?
        } else {
            None
        };
        if route_expected.is_some() && route_witness.is_none() {
            findings.push(InventoryFinding::error(
                "invalid_compatibility_route_wrapper",
                Some(&format!("LEGACY-SKILL:{declared}")),
                Some(&relative(root, &path)?),
                "legacy skill is not the exact explicit-only compatibility wrapper".to_owned(),
            ));
        }
        let (stable_id, authority) = if legacy {
            (format!("LEGACY-SKILL:{declared}"), AuthorityState::Legacy)
        } else {
            (
                format!("SKILL:{declared}"),
                if canonical.contains(&declared) {
                    AuthorityState::Canonical
                } else {
                    AuthorityState::Context
                },
            )
        };
        let kind = if route_witness.is_some() {
            STRUCTURAL_WITNESS_KIND
        } else {
            "skill"
        };
        let provenance = route_witness
            .map(|route| vec![route.metadata_path()])
            .unwrap_or_default();
        let references = route_witness
            .map(|route| vec![route.canonical_id()])
            .unwrap_or_default();
        entries.push(physical_entry(
            reads,
            root,
            &path,
            stable_id,
            kind,
            "OWN-PLUGIN-PRODUCT",
            authority,
            ActiveStatus::Active,
            None,
            provenance,
            references,
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
            record_invalid_metadata(
                reads,
                root,
                &path,
                format!("INVALID-AGENT:{stem}"),
                "agent",
                "OWN-ULTRA-ROOT",
                findings,
                entries,
            )?;
            continue;
        };
        entries.push(physical_entry(
            reads,
            root,
            &path,
            format!("AGENT:{declared}"),
            "agent",
            "OWN-ULTRA-ROOT",
            if required.contains(&declared) {
                AuthorityState::Canonical
            } else {
                AuthorityState::Context
            },
            ActiveStatus::Active,
            None,
            Vec::new(),
            Vec::new(),
        )?);
    }
    Ok(())
}
