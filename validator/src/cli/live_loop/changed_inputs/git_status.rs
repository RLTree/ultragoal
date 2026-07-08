use std::path::Path;

pub(super) fn changed_files(root: &Path) -> Vec<String> {
    if !git_root_matches_requested_root(root) {
        return Vec::new();
    }
    let output = std::process::Command::new("git")
        .args(["status", "--short", "--untracked-files=all"])
        .current_dir(root)
        .output();
    output
        .ok()
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(changed_path)
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn git_root_matches_requested_root(root: &Path) -> bool {
    let requested = root.canonicalize().ok();
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(root)
        .output()
        .ok();
    let git_root = output
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .and_then(|text| Path::new(&text).canonicalize().ok());
    requested
        .zip(git_root)
        .is_some_and(|(requested, git_root)| requested == git_root)
}

pub(crate) fn changed_path(line: &str) -> Option<String> {
    if line.trim().is_empty() {
        return None;
    }
    let path_part = line.get(3..).unwrap_or(line);
    let path = path_part
        .split_once(" -> ")
        .map(|(_, renamed)| renamed)
        .unwrap_or(path_part)
        .trim();
    (!path.is_empty()).then(|| path.to_string())
}
