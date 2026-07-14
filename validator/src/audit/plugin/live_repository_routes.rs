use std::fs;
use std::path::Path;

const REQUIRED_FILES: &[&str] = &[
    "AGENTS.md",
    "AGENT_STANDARDS.md",
    "ARCHITECTURE.md",
    "PLANS.md",
    "SECURITY.md",
    "PRODUCT_SUCCESS_CONTRACT.md",
    "agent-standards/enforcement.json",
    "agent-standards/01-namespace-and-progressive-disclosure.md",
    "agent-standards/02-boundaries-validation-and-enforcement.md",
];

pub(super) fn failures(root: &Path) -> Vec<String> {
    REQUIRED_FILES
        .iter()
        .filter_map(|relative| required_file_failure(root, relative))
        .collect()
}

fn required_file_failure(root: &Path, relative: &str) -> Option<String> {
    let path = root.join(relative);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {
            None
        }
        Ok(_) => Some(format!("live_repository_route_not_regular:{relative}")),
        Err(error) => Some(format!(
            "live_repository_route_missing:{relative}:{error}:templates_are_not_live_authority"
        )),
    }
}
