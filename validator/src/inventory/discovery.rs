use super::fs::{check_symlink, physical_entry, regular_files, relative};
use super::generated;
use super::registry::RegistryData;
use super::schema_references::json_references;
use super::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use crate::context::ReadSession;
use std::path::Path;

fn discovered_references(
    reads: &ReadSession,
    path: &Path,
    id: &str,
    relative: &str,
    findings: &mut Vec<InventoryFinding>,
) -> Vec<String> {
    let scan = json_references(reads, path);
    if let Some(problem) = scan.problem {
        findings.push(InventoryFinding::error(
            "invalid_json_reference_source",
            Some(id),
            Some(relative),
            problem.to_owned(),
        ));
    }
    scan.references
}

fn discover_plugin(
    reads: &ReadSession,
    root: &Path,
    entries: &mut Vec<InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
) -> Result<(), InventoryError> {
    let path = root.join(".codex-plugin/plugin.json");
    if path.exists() {
        let confined = check_symlink(root, &path, findings)?;
        findings.push(InventoryFinding::warning(
            "projection_requires_canonical_reconciliation",
            Some("PLUGIN-MANIFEST"),
            Some(".codex-plugin/plugin.json"),
            "plugin manifest is a product projection, not inventory authority".to_owned(),
        ));
        let references = if confined {
            discovered_references(
                reads,
                &path,
                "PLUGIN-MANIFEST",
                ".codex-plugin/plugin.json",
                findings,
            )
        } else {
            Vec::new()
        };
        entries.push(physical_entry(
            reads,
            root,
            &path,
            "PLUGIN-MANIFEST".to_owned(),
            "plugin-manifest",
            "OWN-PLUGIN-PRODUCT",
            AuthorityState::Projection,
            ActiveStatus::ContextOnly,
            Some("plugin packaging projection".to_owned()),
            Vec::new(),
            references,
        )?);
    } else {
        findings.push(InventoryFinding::error(
            "missing_required_component",
            Some("PLUGIN-MANIFEST"),
            Some(".codex-plugin/plugin.json"),
            "supported plugin manifest is missing".to_owned(),
        ));
    }
    Ok(())
}

struct CollectionSpec<'a> {
    directory: &'a str,
    id_prefix: &'a str,
    kind: &'a str,
    authority: AuthorityState,
}

fn discover_collection(
    reads: &ReadSession,
    root: &Path,
    spec: CollectionSpec<'_>,
    entries: &mut Vec<InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
) -> Result<(), InventoryError> {
    for path in regular_files(reads, root, spec.directory)? {
        let confined = check_symlink(root, &path, findings)?;
        let rel = relative(root, &path)?;
        let references = if spec.kind == "schema" && confined {
            discovered_references(
                reads,
                &path,
                &format!("{}:{rel}", spec.id_prefix),
                &rel,
                findings,
            )
        } else {
            Vec::new()
        };
        entries.push(physical_entry(
            reads,
            root,
            &path,
            format!("{}:{rel}", spec.id_prefix),
            spec.kind,
            "OWN-PRODUCT-ARCHITECTURE",
            spec.authority,
            ActiveStatus::ContextOnly,
            None,
            Vec::new(),
            references,
        )?);
    }
    Ok(())
}

pub(crate) fn discover(
    reads: &ReadSession,
    root: &Path,
    registry: &RegistryData,
) -> Result<DiscoveryData, InventoryError> {
    let mut entries = Vec::new();
    let mut findings = Vec::new();
    discover_plugin(reads, root, &mut entries, &mut findings)?;
    super::components::discover_skills(reads, root, registry, &mut entries, &mut findings)?;
    super::components::discover_agents(reads, root, registry, &mut entries, &mut findings)?;
    for spec in [
        CollectionSpec {
            directory: "schemas",
            id_prefix: "SCHEMA",
            kind: "schema",
            authority: AuthorityState::Context,
        },
        CollectionSpec {
            directory: "fixtures",
            id_prefix: "FIXTURE",
            kind: "fixture",
            authority: AuthorityState::Context,
        },
    ] {
        discover_collection(reads, root, spec, &mut entries, &mut findings)?;
    }
    generated::discover(
        reads,
        root,
        &registry.contract_id,
        &mut entries,
        &mut findings,
    )?;
    Ok(DiscoveryData { entries, findings })
}
use super::component_expectations::DiscoveryData;
