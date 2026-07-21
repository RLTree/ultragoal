use super::{PackageEntry, PackageRole, insert_prefix_free_path};
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use std::collections::BTreeSet;

pub(super) struct Root {
    path: String,
    role: PackageRole,
    subtree: bool,
}

pub(super) fn add_root(
    path: String,
    role: PackageRole,
    subtree: bool,
    roots: &mut Vec<Root>,
    seen: &mut BTreeSet<String>,
) -> Result<(), DistributionError> {
    if !insert_prefix_free_path(seen, &path) {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    roots.push(Root {
        path,
        role,
        subtree,
    });
    Ok(())
}

pub(super) fn bind(entries: &[PackageEntry], roots: &[Root]) -> Result<(), DistributionError> {
    let mut paths = BTreeSet::new();
    if entries
        .iter()
        .any(|entry| !paths.insert(entry.path.to_ascii_lowercase()))
        || !entries.iter().any(|entry| {
            entry.path.starts_with("skills/")
                && entry.path.ends_with("/SKILL.md")
                && entry.role == PackageRole::Skill
        })
        || roots.iter().any(|root| {
            !entries.iter().any(|entry| {
                if root.subtree {
                    entry.path.starts_with(&(root.path.clone() + "/"))
                } else {
                    entry.path == root.path
                }
            })
        })
    {
        return Err(error(DistributionErrorId::ArchiveMismatch));
    }
    for entry in entries
        .iter()
        .filter(|entry| entry.path != ".codex-plugin/plugin.json")
    {
        if entry.path == ".agents/plugins/marketplace.json" && entry.role == PackageRole::Data {
            continue;
        }
        if entry.path == "runtime/runtime-probe-bin" && entry.role == PackageRole::Executable {
            continue;
        }
        let matches = roots
            .iter()
            .filter(|root| {
                entry.path == root.path
                    || (root.subtree && entry.path.starts_with(&(root.path.clone() + "/")))
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 || !role_matches(matches[0], entry) {
            return Err(error(DistributionErrorId::ArchiveMismatch));
        }
    }
    Ok(())
}

fn role_matches(root: &Root, entry: &PackageEntry) -> bool {
    if !root.subtree {
        return root.role == entry.role;
    }
    let relative = entry.path.strip_prefix(&(root.path.clone() + "/"));
    match relative {
        Some(path) if path.ends_with("/SKILL.md") => entry.role == PackageRole::Skill,
        Some(path) if path.contains("/scripts/") => entry.role == PackageRole::Executable,
        Some(path) if path.contains("/agents/") => entry.role == PackageRole::Agent,
        Some(path) if path.contains("/references/") => entry.role == PackageRole::Documentation,
        Some(path) if path.contains("/assets/") => entry.role == PackageRole::Data,
        Some(_) => matches!(entry.role, PackageRole::Data | PackageRole::Documentation),
        None => false,
    }
}
