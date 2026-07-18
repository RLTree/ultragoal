use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const REGISTRY: &str = "migration/generated-surface-authority.json";

pub(crate) fn copy_declared_files(target: &Path) {
    let source = super::repository_fixture::live_root();
    let value: Value = serde_json::from_slice(&fs::read(source.join(REGISTRY)).unwrap()).unwrap();
    let mut paths = BTreeSet::new();
    let projection = value.get("registry_projection").unwrap();
    paths.insert(projection.get("generator").unwrap().as_str().unwrap());
    collect_strings(projection.get("canonical_sources"), &mut paths);
    for surface in value.get("surfaces").unwrap().as_array().unwrap() {
        paths.insert(surface.get("output").unwrap().as_str().unwrap());
        if let Some(generator) = surface.get("generator").and_then(Value::as_str) {
            paths.insert(generator);
        }
        collect_strings(surface.get("canonical_sources"), &mut paths);
    }
    for relative in paths {
        let destination = target.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::copy(source.join(relative), destination).unwrap();
    }
}

fn collect_strings<'a>(value: Option<&'a Value>, paths: &mut BTreeSet<&'a str>) {
    for path in value.and_then(Value::as_array).into_iter().flatten() {
        paths.insert(path.as_str().unwrap());
    }
}
