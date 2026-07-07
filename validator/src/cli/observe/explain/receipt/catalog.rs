use std::path::{Path, PathBuf};

pub(in crate::cli::observe::explain) fn observability_json_files(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_json_files(&root.join("validation_artifacts/observability"), &mut paths);
    paths.sort();
    paths
}

fn collect_json_files(dir: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, paths);
        } else if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            paths.push(path);
        }
    }
}
