use crate::registry_types::{RegistryError, RegistryPayload};
use crate::{
    inventory::{Inventory, safe_path},
    model::*,
};
use serde::de::DeserializeOwned;
use std::{collections::BTreeSet, path::Path};

pub struct Loaded {
    pub sources: Vec<SourceRow>,
    pub dependencies: Vec<DependencyRow>,
    pub boundaries: Vec<BoundaryRow>,
    pub commands: Vec<CommandRow>,
    pub outputs: Vec<OutputRow>,
}

fn read<T: RegistryPayload>(root: &Path, path: &str) -> Result<T, RegistryError> {
    if !safe_path(path) {
        return Err(format!("registry_path_not_exact:{path}").into());
    }
    let full = root.join(path);
    let mut checked = root.to_path_buf();
    for part in Path::new(path).components() {
        checked.push(part);
        if checked
            .symlink_metadata()
            .map_err(|e| format!("registry_read:{path}:{e}"))?
            .file_type()
            .is_symlink()
        {
            return Err(format!("registry_symlink:{path}").into());
        }
    }
    if full
        .symlink_metadata()
        .map_err(|e| format!("registry_read:{path}:{e}"))?
        .file_type()
        .is_symlink()
    {
        return Err(format!("registry_symlink:{path}").into());
    }
    let bytes = std::fs::read(&full).map_err(|e| format!("registry_read:{path}:{e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| RegistryError(format!("registry_parse:{path}:{e}")))
}

fn parts<T: DeserializeOwned>(root: &Path, paths: &[String]) -> Result<Vec<T>, RegistryError>
where
    Vec<T>: RegistryPayload,
{
    let mut seen = BTreeSet::new();
    let mut output = vec![];
    for path in paths {
        if !seen.insert(path) {
            return Err(format!("registry_duplicate_part:{path}").into());
        }
        output.extend(read::<Vec<T>>(root, path)?);
    }
    Ok(output)
}

pub fn load(root: &Path, path: &str) -> Result<Loaded, RegistryError> {
    let registry: Registry = read(root, path)?;
    if registry.schema_version != 1 {
        return Err("registry_schema_version".into());
    }
    let mut dependencies: Vec<DependencyRow> = parts(root, &registry.dependency_registries)?;
    let mut profile_parts = BTreeSet::new();
    for row in &mut dependencies {
        for part in &row.profile_registries {
            if !profile_parts.insert(part.clone()) {
                return Err(format!("dependency_profile_part_duplicate:{part}").into());
            }
        }
        row.profiles
            .extend(parts::<DependencyProfile>(root, &row.profile_registries)?);
    }
    Ok(Loaded {
        sources: parts(root, &registry.source_maps)?,
        dependencies,
        boundaries: parts(root, &registry.boundary_registries)?,
        commands: parts(root, &registry.command_registries)?,
        outputs: parts(root, &registry.output_registries)?,
    })
}

pub fn source_map(rows: &[SourceRow], inventory: &Inventory) -> Vec<String> {
    let mut failures = vec![];
    let mut seen = BTreeSet::new();
    for row in rows {
        if !seen.insert(row.path.as_str()) {
            failures.push(format!("source_map_duplicate:{}", row.path));
        }
        if !safe_path(&row.path) || !inventory.files.contains_key(&row.path) {
            failures.push(format!("source_map_unresolved:{}", row.path));
        }
        if !safe_path(&row.owner) || !inventory.files.contains_key(&row.owner) {
            failures.push(format!(
                "source_owner_unresolved:{}:{}",
                row.path, row.owner
            ));
        }
        if row.purpose.trim().is_empty() {
            failures.push(format!("source_purpose_missing:{}", row.path));
        }
        if !matches!(
            row.class.as_str(),
            "rust_source"
                | "test"
                | "script"
                | "interface"
                | "guidance"
                | "active_document"
                | "manifest"
                | "governance"
                | "verified_generated_lock"
        ) {
            failures.push(format!("source_class_invalid:{}:{}", row.path, row.class));
        }
        if row.class == "verified_generated_lock" && !row.path.ends_with("Cargo.lock") {
            failures.push(format!("generated_class_not_lock:{}", row.path));
        }
        if row.class == "script" {
            failures.push(format!("semantic_coverage_unsupported:{}", row.path));
        }
        if row.class == "rust_source" && !row.path.ends_with(".rs") {
            failures.push(format!("rust_source_class_not_rust:{}", row.path));
        }
    }
    for path in inventory.files.keys() {
        if !seen.contains(path.as_str()) {
            failures.push(format!("source_map_missing:{path}"));
        }
    }
    failures
}

pub fn commands(rows: &[CommandRow], inventory: &Inventory, targets: &[String]) -> Vec<String> {
    let mut failures = vec![];
    let mut ids = BTreeSet::new();
    if rows.is_empty() {
        failures.push("command_ownership_empty".into());
    }
    for row in rows {
        if row.id.trim().is_empty() || !ids.insert(&row.id) {
            failures.push(format!("command_identity_invalid:{}", row.id));
        }
        for path in [&row.owner, &row.entrypoint] {
            if !safe_path(path) || !inventory.files.contains_key(path) {
                failures.push(format!("command_owner_unresolved:{}:{path}", row.id));
            }
        }
        let valid = match row.kind.as_str() {
            "cargo" => {
                row.entrypoint.ends_with("Cargo.toml")
                    && row.argv.first().is_some_and(|s| s == "cargo")
                    && row.argv.len() > 1
            }
            "python" => {
                row.entrypoint.ends_with(".py")
                    && row.argv.get(1) == Some(&row.entrypoint)
                    && row.argv.first().is_some_and(|s| s == "python3")
            }
            "binary" => targets.contains(&row.entrypoint) && !row.argv.is_empty(),
            _ => false,
        };
        if !valid {
            failures.push(format!("command_entrypoint_mismatch:{}", row.id));
        }
    }
    failures
}
