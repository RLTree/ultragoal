use super::manifest_bind::{Root, add_root, bind};
use super::plan::PackageEntry;
use super::spec::PackageRole;
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::reader::validate_relative_path;
use crate::plugin_manifest::{self, DecodeError};
use std::collections::BTreeSet;

pub(super) fn validate(
    entries: &[PackageEntry],
    plugin_id: &str,
    version: &str,
) -> Result<(), DistributionError> {
    let entry = entries
        .iter()
        .find(|entry| entry.path == ".codex-plugin/plugin.json")
        .ok_or_else(|| error(DistributionErrorId::ArchiveMismatch))?;
    let manifest = plugin_manifest::parse(&entry.bytes, plugin_manifest::MANIFEST_LIMIT)
        .map_err(manifest_error)?;
    if !plugin_manifest::semantic_issues(&manifest).is_empty() {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    metadata(&manifest, plugin_id, version)?;
    let mut roots = Vec::new();
    let mut seen = BTreeSet::new();
    let skills = component(
        manifest
            .skills
            .as_deref()
            .ok_or_else(|| error(DistributionErrorId::InvalidSpec))?,
        true,
    )?;
    if skills != "skills" {
        return Err(error(DistributionErrorId::InvalidPath));
    }
    add_root(skills, PackageRole::Skill, true, &mut roots, &mut seen)?;
    if let Some(path) = manifest.apps.as_deref() {
        let path = component(path, false)?;
        if path != ".app.json" {
            return Err(error(DistributionErrorId::InvalidPath));
        }
        add_root(path, PackageRole::Data, false, &mut roots, &mut seen)?;
    }
    if let Some(servers) = &manifest.mcp_servers {
        validate_mcp(servers, &mut roots, &mut seen)?;
    }
    if let Some(interface) = &manifest.interface {
        validate_interface(interface, &mut roots, &mut seen)?;
    }
    bind(entries, &roots)
}

fn manifest_error(error_kind: DecodeError) -> DistributionError {
    error(match error_kind {
        DecodeError::ObjectTooLarge => DistributionErrorId::ObjectTooLarge,
        DecodeError::InvalidJson => DistributionErrorId::InvalidJson,
        DecodeError::InvalidShape => DistributionErrorId::InvalidSpec,
    })
}

fn metadata(
    manifest: &plugin_manifest::PluginManifest,
    plugin_id: &str,
    version: &str,
) -> Result<(), DistributionError> {
    if manifest.name != plugin_id || manifest.version != version {
        return Err(error(DistributionErrorId::ArchiveMismatch));
    }
    Ok(())
}

fn validate_interface(
    value: &plugin_manifest::PluginInterface,
    roots: &mut Vec<Root>,
    seen: &mut BTreeSet<String>,
) -> Result<(), DistributionError> {
    for asset in [&value.composer_icon, &value.logo, &value.logo_dark]
        .into_iter()
        .flatten()
    {
        add_asset(asset, false, roots, seen)?;
    }
    if value.screenshots.len() > plugin_manifest::ITEM_LIMIT {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    for asset in &value.screenshots {
        add_asset(asset, true, roots, seen)?;
    }
    Ok(())
}

fn validate_mcp(
    value: &plugin_manifest::PluginMcpServers,
    roots: &mut Vec<Root>,
    seen: &mut BTreeSet<String>,
) -> Result<(), DistributionError> {
    match value {
        plugin_manifest::PluginMcpServers::Path(path) => {
            let path = component(path, false)?;
            if path != ".mcp.json" {
                return Err(error(DistributionErrorId::InvalidPath));
            }
            add_root(path, PackageRole::Data, false, roots, seen)
        }
        plugin_manifest::PluginMcpServers::Inline(servers) => {
            for server in servers.values() {
                if let Some(command) = server
                    .command
                    .as_deref()
                    .filter(|row| row.starts_with("./"))
                {
                    add_root(
                        component(command, false)?,
                        PackageRole::Executable,
                        false,
                        roots,
                        seen,
                    )?;
                } else if server.command.is_some() {
                    return Err(error(DistributionErrorId::InvalidSpec));
                }
            }
            Ok(())
        }
    }
}

fn add_asset(
    value: &str,
    png: bool,
    roots: &mut Vec<Root>,
    seen: &mut BTreeSet<String>,
) -> Result<(), DistributionError> {
    let path = component(value, false)?;
    if !path.starts_with("assets/") || (png && !path.ends_with(".png")) {
        return Err(error(DistributionErrorId::InvalidPath));
    }
    add_root(path, PackageRole::Data, false, roots, seen)
}

fn component(value: &str, subtree: bool) -> Result<String, DistributionError> {
    let raw = value
        .strip_prefix("./")
        .ok_or_else(|| error(DistributionErrorId::InvalidPath))?;
    let path = if subtree {
        raw.strip_suffix('/').unwrap_or(raw)
    } else {
        raw
    };
    if path.is_empty() || (!subtree && path.ends_with('/')) {
        return Err(error(DistributionErrorId::InvalidPath));
    }
    validate_relative_path(path)?;
    Ok(path.to_owned())
}
