use super::insert_prefix_free_path;
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::json;
use crate::distribution::reader::validate_relative_path;
use crate::distribution::spec::digest;
use crate::plugin_manifest::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const SPEC_LIMIT: usize = 1024 * 1024;
pub(crate) const ENTRY_LIMIT: usize = 4 * 1024 * 1024;
pub(crate) const CLI_ENTRY_LIMIT: usize = 32 * 1024 * 1024;
pub(crate) const PACKAGE_LIMIT: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageRole {
    Manifest,
    Skill,
    Agent,
    Documentation,
    Executable,
    Data,
}

impl PackageRole {
    pub(crate) const fn code(self) -> u8 {
        match self {
            Self::Manifest => 1,
            Self::Skill => 2,
            Self::Agent => 3,
            Self::Documentation => 4,
            Self::Executable => 5,
            Self::Data => 6,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackageSpec {
    schema: String,
    context_id: String,
    candidate_id: String,
    plugin_id: String,
    version: String,
    source_date_epoch: u64,
    entries: Vec<RawEntry>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    path: String,
    source_path: String,
    role: PackageRole,
    executable: bool,
}

pub(crate) struct PackageSpec {
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) plugin_id: String,
    pub(crate) version: String,
    pub(crate) source_date_epoch: u64,
    pub(crate) entries: Vec<PackageSpecEntry>,
}

pub(crate) struct PackageSpecEntry {
    pub(crate) path: String,
    pub(crate) source_path: String,
    pub(crate) role: PackageRole,
    pub(crate) mode: u32,
}

pub(crate) fn parse(bytes: &[u8]) -> Result<PackageSpec, DistributionError> {
    let raw: RawPackageSpec = json::parse(bytes, SPEC_LIMIT)?;
    if raw.schema != "harness-ultragoal.package-plan.v1"
        || !digest(&raw.context_id)
        || !digest(&raw.candidate_id)
        || raw.plugin_id != "harness-ultragoal"
        || Version::parse(&raw.version).is_none()
        || raw.entries.is_empty()
        || raw.entries.len() > 4096
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    let mut targets = BTreeSet::new();
    let mut entries = Vec::with_capacity(raw.entries.len());
    for row in raw.entries {
        validate_relative_path(&row.path)?;
        validate_relative_path(&row.source_path)?;
        if !insert_prefix_free_path(&mut targets, &row.path)
            || forbidden_runtime_path(&row.path)
            || forbidden_runtime_path(&row.source_path)
            || (row.role == PackageRole::Executable) != row.executable
            || (row.path == ".codex-plugin/plugin.json") != (row.role == PackageRole::Manifest)
        {
            return Err(error(DistributionErrorId::InvalidSpec));
        }
        entries.push(PackageSpecEntry {
            path: row.path,
            source_path: row.source_path,
            role: row.role,
            mode: if row.executable { 0o755 } else { 0o644 },
        });
    }
    if !targets.contains(".codex-plugin/plugin.json") {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(PackageSpec {
        context_id: raw.context_id,
        candidate_id: raw.candidate_id,
        plugin_id: raw.plugin_id,
        version: raw.version,
        source_date_epoch: raw.source_date_epoch,
        entries,
    })
}

fn forbidden_runtime_path(path: &str) -> bool {
    path == ".DS_Store"
        || path.ends_with("/.DS_Store")
        || path == ".git"
        || path.starts_with(".git/")
        || path
            .split('/')
            .any(|component| component == ".codex-worktree")
        || path.starts_with("target/")
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_state_is_forbidden_as_package_target_or_source() {
        assert!(super::forbidden_runtime_path(".codex-worktree"));
        assert!(super::forbidden_runtime_path(".codex-worktree/env.sh"));
        assert!(super::forbidden_runtime_path(".codex-worktree/run-command"));
        assert!(super::forbidden_runtime_path(
            ".codex-worktree/nested/private"
        ));
        assert!(super::forbidden_runtime_path(
            "nested/.codex-worktree/env.sh"
        ));
        assert!(!super::forbidden_runtime_path(
            ".codex/environments/environment.toml"
        ));
    }
}
