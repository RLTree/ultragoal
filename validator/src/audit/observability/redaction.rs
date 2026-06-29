use std::fs;
use std::path::Path;

const OBSERVABILITY_ARTIFACT_ROOT: &str = "validation_artifacts/observability";

pub(super) fn check(root: &Path, out: &mut Vec<String>) {
    let artifact_root = root.join(OBSERVABILITY_ARTIFACT_ROOT);
    if !artifact_root.exists() {
        return;
    }
    scan_dir(root, &artifact_root, out);
}

fn scan_dir(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        out.push(format!(
            "observability_artifact_read_error:{}",
            rel(root, dir)
        ));
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(root, &path, out);
        } else {
            scan_file(root, &path, out);
        }
    }
}

fn scan_file(root: &Path, path: &Path, out: &mut Vec<String>) {
    let rel = rel(root, path);
    let Ok(text) = fs::read_to_string(path) else {
        out.push(format!("observability_artifact_read_error:{rel}"));
        return;
    };
    if leaked_private_path(&text) {
        out.push(format!("observability_private_path_leak:{rel}"));
    }
    if leaked_secret_marker(&text) {
        out.push(format!("observability_secret_marker_leak:{rel}"));
    }
}

fn leaked_private_path(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    contains_marker(&lower, &private_path_markers())
}

fn private_path_markers() -> [&'static str; 6] {
    [
        "/users/",
        "file:///users/",
        "unix:///users/",
        "/private/tmp/",
        "file:///private/tmp/",
        "unix:///private/tmp/",
    ]
}

fn leaked_secret_marker(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    contains_marker(
        &lower,
        &[
            "authorization:",
            "api_key",
            "token=",
            "cookie",
            "database_url",
        ],
    )
}

fn contains_marker(text: &str, markers: &[&str]) -> bool {
    for marker in markers {
        if text.contains(marker) {
            return true;
        }
    }
    false
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
