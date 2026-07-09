use std::path::{Component, Path, PathBuf};

pub(super) fn safe_observation_receipt_path(root: &Path, rel: &str) -> Option<PathBuf> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute()
        || !rel.starts_with("validation_artifacts/observability/live-loop/commands/")
        || !rel.ends_with("-command-observation.json")
        || rel_path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return None;
    }
    let root = root.canonicalize().ok()?;
    let mut path = root.clone();
    for component in rel_path.components() {
        let Component::Normal(name) = component else {
            return None;
        };
        path.push(name);
        if std::fs::symlink_metadata(&path)
            .ok()?
            .file_type()
            .is_symlink()
        {
            return None;
        }
    }
    let canonical = path.canonicalize().ok()?;
    canonical.starts_with(&root).then_some(canonical)
}

#[cfg(test)]
pub(super) fn observation_receipt_path_for_tests(root: &Path, rel: &str) -> Option<PathBuf> {
    safe_observation_receipt_path(root, rel)
}
