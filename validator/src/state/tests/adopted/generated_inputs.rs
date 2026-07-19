use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};

const REGISTRY: &str = "migration/generated-surface-authority.json";

pub(super) fn copy_generated_inputs(live: &Path, root: &Path) {
    let registry: Value = serde_json::from_slice(&fs::read(live.join(REGISTRY)).unwrap()).unwrap();
    let mut paths = BTreeSet::new();
    let projection = registry.get("registry_projection").unwrap();
    insert_path(projection.get("generator"), &mut paths);
    let shards = paths_from(projection.get("canonical_sources"));
    paths.extend(shards.iter().cloned());
    for shard in shards {
        let value: Value = serde_json::from_slice(&fs::read(live.join(&shard)).unwrap()).unwrap();
        for surface in value.get("surfaces").unwrap().as_array().unwrap() {
            insert_path(surface.get("output"), &mut paths);
            insert_path(surface.get("generator"), &mut paths);
            insert_path(surface.get("schema"), &mut paths);
            insert_path(surface.get("source_contract"), &mut paths);
            insert_path(surface.get("amendment_log"), &mut paths);
            paths.extend(paths_from(surface.get("canonical_sources")));
        }
    }
    for path in paths {
        let relative = Path::new(&path);
        assert!(relative
            .components()
            .all(|part| matches!(part, Component::Normal(_))));
        let destination = root.join(relative);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(live.join(relative), destination).unwrap();
    }
}

fn insert_path(value: Option<&Value>, paths: &mut BTreeSet<String>) {
    if let Some(path) = value.and_then(Value::as_str) {
        paths.insert(path.to_owned());
    }
}

fn paths_from(value: Option<&Value>) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for path in value.and_then(Value::as_array).into_iter().flatten() {
        insert_path(Some(path), &mut paths);
    }
    paths
}
