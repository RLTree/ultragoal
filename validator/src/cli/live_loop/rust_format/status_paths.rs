use std::path::{Component, Path, PathBuf};

pub(super) fn changed_paths_from_status(status: &str) -> Vec<PathBuf> {
    if status.contains('\0') {
        return changed_paths_from_porcelain_z(status);
    }
    status.lines().flat_map(changed_paths).collect()
}

fn changed_paths_from_porcelain_z(status: &str) -> Vec<PathBuf> {
    let mut entries = status.split('\0').filter(|entry| !entry.is_empty());
    let mut out = Vec::new();
    while let Some(entry) = entries.next() {
        let status = entry.get(..2).unwrap_or("");
        let path_part = entry.get(3..).unwrap_or(entry);
        let mut paths = trimmed_path(path_part).into_iter().collect::<Vec<_>>();
        if status.contains('R') || status.contains('C') {
            paths.extend(entries.next().and_then(trimmed_path));
        }
        if status.contains('D') {
            out.extend(paths.into_iter().filter(|path| is_rustfmt_config(path)));
        } else {
            out.extend(paths);
        }
    }
    out
}

fn changed_paths(line: &str) -> Vec<PathBuf> {
    if line.trim().is_empty() {
        return Vec::new();
    }
    let status = line.get(..2).unwrap_or("");
    let path_part = line.get(3..).unwrap_or(line);
    let paths = rename_paths(path_part);
    if status.contains('D') {
        return paths
            .into_iter()
            .filter(|path| is_rustfmt_config(path))
            .collect();
    }
    paths
}

fn rename_paths(path_part: &str) -> Vec<PathBuf> {
    match path_part.split_once(" -> ") {
        Some((source, destination)) => [source, destination]
            .into_iter()
            .filter_map(trimmed_path)
            .collect(),
        None => trimmed_path(path_part).into_iter().collect(),
    }
}

fn trimmed_path(path: &str) -> Option<PathBuf> {
    let path = path.trim();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

pub(super) fn is_rust_source(path: &Path) -> bool {
    path.starts_with("validator/") && path.extension().and_then(|ext| ext.to_str()) == Some("rs")
}

pub(super) fn is_rustfmt_config(path: &Path) -> bool {
    matches!(
        path.to_str(),
        Some(
            "rustfmt.toml" | ".rustfmt.toml" | "validator/rustfmt.toml" | "validator/.rustfmt.toml"
        )
    )
}

pub(super) fn is_package_owned_rustfmt_input(root: &Path, rel: &Path) -> bool {
    if rel.is_absolute()
        || rel
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return false;
    }
    std::fs::symlink_metadata(root.join(rel))
        .map(|metadata| metadata.file_type().is_file())
        .unwrap_or(false)
}
