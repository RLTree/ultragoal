use super::fs::{PhysicalEntryDescriptor, check_symlink, physical_regular_entry};
use super::plugin_manifest;
use super::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use crate::context::ReadSession;
use std::path::Path;

const DEFAULT_PATH: &str = "hooks/hooks.json";
const DEFAULT_DECLARATION: &str = "./hooks/hooks.json";

fn finding(
    inspection: &super::plugin_manifest_hooks::Inspection,
    id: &str,
    path: &str,
    findings: &mut Vec<InventoryFinding>,
) {
    if !inspection.valid {
        findings.push(InventoryFinding::error(
            "invalid_plugin_hooks",
            Some(id),
            Some(path),
            "plugin hook document is malformed or unsafe".to_owned(),
        ));
        return;
    }
    if inspection.inactive {
        findings.push(InventoryFinding::warning(
            "inactive_plugin_hook_configuration",
            Some(id),
            Some(path),
            "plugin hook document contains configuration the current host parses but skips"
                .to_owned(),
        ));
    }
    if inspection.trust_required {
        findings.push(InventoryFinding::warning(
            "plugin_hook_trust_required",
            Some(id),
            Some(path),
            "plugin command hooks require current host trust before they can run".to_owned(),
        ));
    }
}

fn discover_file(
    reads: &ReadSession,
    root: &Path,
    declaration: &str,
    source: &str,
    validate: bool,
    entries: &mut Vec<InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
) -> Result<(), InventoryError> {
    let relative = super::plugin_manifest_path::relative(declaration)
        .ok_or_else(|| InventoryError::InvalidRegistry("invalid plugin hook path".to_owned()))?;
    let path = root.join(relative);
    if std::fs::symlink_metadata(&path).is_err() {
        return Ok(());
    }
    let confined = check_symlink(root, &path, findings)?;
    if !confined {
        return Ok(());
    }
    let metadata = std::fs::symlink_metadata(&path).map_err(|error| InventoryError::Io {
        path: path.clone(),
        message: error.to_string(),
    })?;
    let id = format!("PLUGIN-HOOKS:{relative}");
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        findings.push(InventoryFinding::error(
            "plugin_hook_file_unsupported",
            Some(&id),
            Some(relative),
            "plugin hook source must be a regular confined non-symlink file".to_owned(),
        ));
        return Ok(());
    }
    let inspection = super::plugin_manifest_hook_document::inspect(reads, root, declaration);
    if validate {
        finding(&inspection, &id, relative, findings);
    } else {
        findings.push(InventoryFinding::warning(
            "plugin_default_hooks_shadowed",
            Some(&id),
            Some(relative),
            "default plugin hooks are present but shadowed by the manifest hooks declaration"
                .to_owned(),
        ));
    }
    let status = if validate && inspection.trust_required {
        ActiveStatus::Candidate
    } else {
        ActiveStatus::ContextOnly
    };
    entries.push(physical_regular_entry(
        reads,
        root,
        &path,
        PhysicalEntryDescriptor {
            stable_id: id,
            kind: "plugin-hooks",
            owner: "OWN-PLUGIN-PRODUCT",
            authority_state: AuthorityState::Projection,
            active_status: status,
            generator: None,
            provenance: vec![
                "plugin-manifest:.codex-plugin/plugin.json".to_owned(),
                format!("hook-source:{source}"),
            ],
            references: vec!["host-hook-trust:unconfirmed".to_owned()],
        },
    )?);
    Ok(())
}

pub(super) fn discover(
    reads: &ReadSession,
    root: &Path,
    manifest: &plugin_manifest::Inspection,
    entries: &mut Vec<InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
) -> Result<(), InventoryError> {
    for path in &manifest.hook_paths {
        discover_file(
            reads,
            root,
            path,
            "manifest-explicit",
            true,
            entries,
            findings,
        )?;
    }
    let default = root.join(DEFAULT_PATH);
    if !manifest
        .hook_paths
        .iter()
        .any(|path| path == DEFAULT_DECLARATION)
        && std::fs::symlink_metadata(default).is_ok()
    {
        discover_file(
            reads,
            root,
            DEFAULT_DECLARATION,
            "host-default",
            !manifest.hooks_declared,
            entries,
            findings,
        )?;
    }
    Ok(())
}
