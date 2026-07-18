use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const REGISTRY: &str = "migration/generated-surface-authority.json";

pub(super) fn copy_declared_files(live: &Path, root: &Path) {
    let registry_path = live.join(REGISTRY);
    let registry: Value = serde_json::from_slice(&fs::read(&registry_path).unwrap()).unwrap();
    let target_registry = root.join(REGISTRY);
    fs::create_dir_all(target_registry.parent().unwrap()).unwrap();
    fs::copy(&registry_path, target_registry).unwrap();
    let mut paths = BTreeSet::new();
    let projection = registry.get("registry_projection").unwrap();
    insert_path(projection.get("generator"), &mut paths);
    let shard_paths = paths_from(projection.get("canonical_sources"));
    paths.extend(shard_paths.iter().cloned());
    for shard in shard_paths {
        let value: Value = serde_json::from_slice(&fs::read(live.join(&shard)).unwrap()).unwrap();
        for surface in value.get("surfaces").unwrap().as_array().unwrap() {
            insert_path(surface.get("output"), &mut paths);
            insert_path(surface.get("generator"), &mut paths);
            insert_paths(surface.get("canonical_sources"), &mut paths);
        }
    }
    for path in paths {
        let target = root.join(&path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(live.join(path), target).unwrap();
    }
}

fn insert_path(value: Option<&Value>, paths: &mut BTreeSet<String>) {
    if let Some(path) = value.and_then(Value::as_str) {
        paths.insert(path.to_owned());
    }
}

fn insert_paths(value: Option<&Value>, paths: &mut BTreeSet<String>) {
    paths.extend(paths_from(value));
}

fn paths_from(value: Option<&Value>) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for path in value.and_then(Value::as_array).into_iter().flatten() {
        insert_path(Some(path), &mut paths);
    }
    paths
}
