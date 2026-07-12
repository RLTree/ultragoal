use crate::audit::contract::{Failure, REQUIRED_SKILLS};
use crate::claim_semantics::str_field;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[cfg(test)]
pub fn check_plugin(bundle: &Value, root: &Path, out: &mut Vec<Failure>) {
    let manifest = &bundle["plugin_manifest"];
    check_plugin_manifest(manifest, root, out);
}

pub(crate) fn check_plugin_with_cache(
    bundle: &Value,
    root: &Path,
    out: &mut Vec<Failure>,
    cache: &mut BTreeMap<String, Vec<Failure>>,
) {
    let manifest = &bundle["plugin_manifest"];
    let digest = crate::digest::canonical_json(manifest);
    if let Some(cached) = cache.get(&digest) {
        out.extend(cached.clone());
        return;
    }
    let start = out.len();
    check_plugin_manifest(manifest, root, out);
    cache.insert(digest, out[start..].to_vec());
}

fn check_plugin_manifest(manifest: &Value, root: &Path, out: &mut Vec<Failure>) {
    let plugin_paths = crate::package::inventory::inventory_paths(manifest)
        .into_iter()
        .filter(|path| !legacy_agent_path(path))
        .collect::<Vec<_>>();
    super::retired_reviewer_policy::check_packaged_paths(&plugin_paths, out);
    for failure in crate::package::resource::purpose::failures(root, manifest) {
        out.push(Failure::new(
            "plugin-inventory-closure",
            failure.code,
            failure.detail,
        ));
    }
    for path in &plugin_paths {
        if let Some(error) = crate::package::inventory::package_path_error(root, path) {
            out.push(Failure::new(
                "plugin-inventory-closure",
                "plugin_path_invalid",
                format!("{path}: {error}"),
            ));
        } else if crate::package::inventory::resolve(root, path)
            .map(|p| !p.exists())
            .unwrap_or(true)
        {
            out.push(Failure::new(
                "plugin-inventory-closure",
                "plugin_agent_path_missing",
                path,
            ));
        }
    }
    required_skill_checks(manifest, out);
    required_agent_checks(manifest, root, out);
    for failure in crate::skill_links::manifest_failures(root, manifest) {
        out.push(Failure::new(
            "skill-inventory-closure",
            failure.code,
            failure.detail,
        ));
    }
}

fn legacy_agent_path(path: &str) -> bool {
    path.starts_with("agents/") || path.starts_with("custom-agents/")
}

fn required_skill_checks(manifest: &Value, out: &mut Vec<Failure>) {
    let skills = manifest
        .get("skills")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|row| str_field(row, "name"))
        .collect::<BTreeSet<_>>();
    for required in REQUIRED_SKILLS {
        if !skills.contains(*required) {
            out.push(Failure::new(
                "skill-inventory-closure",
                "required_skill_missing",
                *required,
            ));
        }
    }
}

fn required_agent_checks(manifest: &Value, root: &Path, out: &mut Vec<Failure>) {
    let agents = manifest
        .get("agents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let roles = crate::agent_roles::CANONICAL_AGENT_ROLES;
    if agents.len() != roles.len() {
        out.push(Failure::new(
            "plugin-inventory-closure",
            "canonical_agent_count_mismatch",
            agents.len().to_string(),
        ));
    }
    for role in roles {
        let matches = agents
            .iter()
            .filter(|row| str_field(row, "name") == role.name)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            out.push(Failure::new(
                "plugin-inventory-closure",
                "canonical_agent_missing",
                role.name,
            ));
            continue;
        }
        let path = str_field(matches[0], "path");
        if path != role.manifest_path {
            out.push(Failure::new(
                "plugin-inventory-closure",
                "canonical_agent_path_mismatch",
                format!("{}:{path}", role.name),
            ));
            continue;
        }
        inspect_agent_manifest(root, role, out);
    }
    for row in agents {
        let name = str_field(row, "name");
        if crate::agent_roles::by_name(&name).is_none() {
            out.push(Failure::new(
                "plugin-inventory-closure",
                "unexpected_agent_role",
                name,
            ));
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectAgentManifest {
    name: String,
    description: String,
    developer_instructions: String,
    sandbox_mode: String,
}

fn inspect_agent_manifest(
    root: &Path,
    role: crate::agent_roles::AgentRole,
    out: &mut Vec<Failure>,
) {
    let result = crate::package::inventory::resolve(root, role.manifest_path)
        .map_err(|error| error.to_string())
        .and_then(|path| crate::digest::read_file_bytes(&path).map_err(|error| error.to_string()))
        .and_then(|bytes| String::from_utf8(bytes).map_err(|error| error.to_string()))
        .and_then(|text| {
            toml::from_str::<ProjectAgentManifest>(&text).map_err(|error| error.to_string())
        });
    let valid = result.is_ok_and(|manifest| {
        manifest.name == role.name
            && !manifest.description.trim().is_empty()
            && !manifest.developer_instructions.trim().is_empty()
            && manifest.sandbox_mode == "read-only"
    });
    if !valid {
        out.push(Failure::new(
            "plugin-inventory-closure",
            "canonical_agent_manifest_invalid",
            role.manifest_path,
        ));
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
