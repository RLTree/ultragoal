use std::path::Path;

const EXCLUDED_SUBTREES: &[&str] = &[
    ".git/codex-target",
    ".git/objects",
    "node_modules",
    "target",
    "validator/target",
    "vendor",
];

pub(super) fn excluded(relative: &Path) -> bool {
    EXCLUDED_SUBTREES
        .iter()
        .any(|value| relative == Path::new(value))
}
