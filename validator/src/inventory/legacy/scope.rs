use std::path::Path;

pub(super) fn inspect(rel: &Path) -> bool {
    let lower = rel.to_string_lossy().to_ascii_lowercase();
    !canonical_collection(&lower)
        && !lower.starts_with("validator/tests/")
        && !lower.starts_with("validator/src/self_tests/")
        && !lower.starts_with("state/codex-review-artifacts/")
        && lower != "validator/src/inventory/legacy.rs"
        && !lower.starts_with("validator/src/inventory/legacy/")
}

fn canonical_collection(lower: &str) -> bool {
    lower.starts_with("skills/")
        || lower.starts_with("schemas/")
        || lower.starts_with("fixtures/")
        || lower.starts_with(".codex/agents/")
        || lower.starts_with("generated/")
        || lower.starts_with("docs/generated/")
        || lower.starts_with("examples/generated/")
        || lower.starts_with("docs/ultragoal-contract-2026-07-successor-v2/")
}
