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
    add_canonical_agent_roots(entries, &mut roots, &mut seen)?;
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

fn add_canonical_agent_roots(
    entries: &[PackageEntry],
    roots: &mut Vec<Root>,
    seen: &mut BTreeSet<String>,
) -> Result<(), DistributionError> {
    if !entries.iter().any(|entry| {
        entry
            .path
            .to_ascii_lowercase()
            .starts_with(".codex/agents/")
    }) {
        return Ok(());
    }
    for role in crate::agent_roles::CANONICAL_AGENT_ROLES {
        add_root(
            role.manifest_path.to_owned(),
            PackageRole::Agent,
            false,
            roots,
            seen,
        )?;
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    const VERSION: &str = "0.0.41+codex.20260824093100";

    fn entry(path: &str, role: PackageRole, bytes: &[u8]) -> PackageEntry {
        PackageEntry {
            path: path.to_owned(),
            mode: 0o644,
            role,
            sha256: String::new(),
            bytes: bytes.to_vec(),
        }
    }

    fn entries() -> Vec<PackageEntry> {
        let mut entries = vec![
            entry(
                ".codex-plugin/plugin.json",
                PackageRole::Manifest,
                br#"{"name":"harness-ultragoal","version":"0.0.41+codex.20260824093100","description":"valid","skills":"./skills/"}"#,
            ),
            entry(
                "skills/harness-ultragoal/SKILL.md",
                PackageRole::Skill,
                b"---\nname: harness-ultragoal\n---\n",
            ),
        ];
        entries.extend(
            crate::agent_roles::CANONICAL_AGENT_ROLES
                .iter()
                .map(|role| entry(role.manifest_path, PackageRole::Agent, role.name.as_bytes())),
        );
        entries
    }

    #[test]
    fn canonical_root_agents_are_exact_non_subtree_agent_roots() {
        let entries = entries();
        validate(&entries, "harness-ultragoal", VERSION).expect("canonical root agents");

        let mut missing = entries.clone();
        missing.retain(|entry| {
            entry.path != crate::agent_roles::CANONICAL_AGENT_ROLES[0].manifest_path
        });
        assert!(validate(&missing, "harness-ultragoal", VERSION).is_err());

        let mut nested = entries.clone();
        nested.push(entry(
            ".codex/agents/nested/member.toml",
            PackageRole::Agent,
            b"unknown",
        ));
        assert!(validate(&nested, "harness-ultragoal", VERSION).is_err());

        let mut wrong_role = entries;
        wrong_role
            .iter_mut()
            .find(|entry| entry.path == crate::agent_roles::CANONICAL_AGENT_ROLES[0].manifest_path)
            .expect("canonical agent")
            .role = PackageRole::Data;
        assert!(validate(&wrong_role, "harness-ultragoal", VERSION).is_err());
    }
}
