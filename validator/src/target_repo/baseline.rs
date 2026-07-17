use crate::digest;
use serde_json::Value;
use std::path::Path;

pub const BASELINE_FILES: &[&str] = &[
    "AGENTS.md",
    "AGENT_STANDARDS.md",
    "ARCHITECTURE.md",
    "PLANS.md",
    "DESIGN.md",
    "FRONTEND.md",
    "PRODUCT_SENSE.md",
    "QUALITY_SCORE.md",
    "RELIABILITY.md",
    "SECURITY.md",
    ".gitignore",
    ".codex/setup-worktree-env.sh",
    ".codex/environments/environment.toml",
    ".harness/coverage-command",
    ".harness/coverage-manifest.json",
    "COVERAGE_RECEIPT.json",
    "agent-standards/enforcement.json",
    "agent-standards/enforcement.tsv",
    "agent-standards/enforcement-audit.tsv",
    "scripts/check-agent-standards",
    "scripts/check-coverage-fast",
    "scripts/check-coverage-full",
    "validation_artifacts/harness/fit-repo-receipt.json",
    "docs/plugin-cohesion-manifest.json",
    "docs/design-docs/index.md",
    "docs/design-docs/core-beliefs.md",
    "docs/exec-plans/tech-debt-tracker.md",
    "docs/generated/index.md",
    "docs/product-specs/index.md",
    "docs/references/index.md",
];
pub const BASELINE_DIRS: &[&str] = &[
    "docs/design-docs",
    "docs/exec-plans/active",
    "docs/exec-plans/completed",
    "docs/generated",
    "docs/product-specs",
    "docs/references",
    "validation_artifacts",
];

pub fn baseline_checks(repo: &Path, checks: &mut serde_json::Map<String, Value>) {
    for rel in BASELINE_FILES {
        let ok = baseline_file_ok(repo, rel);
        checks.insert(
            format!("baseline-file:{rel}"),
            crate::target_repo::row(
                repo,
                if ok { "pass" } else { "fail" },
                baseline_detail(ok),
                Some(rel),
            ),
        );
    }
    for rel in BASELINE_DIRS {
        let ok = crate::target_repo::safe_fs::is_dir(repo, rel);
        checks.insert(
            format!("baseline-dir:{rel}"),
            crate::target_repo::row(
                repo,
                if ok { "pass" } else { "fail" },
                "required directory",
                Some(rel),
            ),
        );
    }
}

fn baseline_file_ok(repo: &Path, rel: &str) -> bool {
    if rel == ".gitignore" {
        return crate::target_repo::safe_fs::read_to_string(repo, rel)
            .is_ok_and(|text| text.lines().any(|line| line.trim() == ".codex-worktree/"));
    }
    if rel == ".codex/environments/environment.toml" {
        let Ok(text) = crate::target_repo::safe_fs::read_to_string(repo, rel) else {
            return false;
        };
        return super::worktree_env_contract::text_ok(&text);
    }
    if rel == ".harness/coverage-command" {
        return coverage_command_ok(repo, rel);
    }
    non_placeholder(repo, rel)
}

fn coverage_command_ok(repo: &Path, rel: &str) -> bool {
    let Ok(text) = crate::target_repo::safe_fs::read_to_string(repo, rel) else {
        return false;
    };
    let lower = text.to_ascii_lowercase();
    !text.trim().is_empty()
        && !lower.contains("setup blocker")
        && !lower.contains("replace .harness/coverage-command")
        && !lower.contains("exit 2")
}

pub fn agent_surface_check(repo: &Path, checks: &mut serde_json::Map<String, Value>) {
    let rel = [".agents/skills", ".codex/skills"]
        .into_iter()
        .find(|p| crate::target_repo::safe_fs::is_dir(repo, p));
    checks.insert(
        "agent-surface".to_string(),
        crate::target_repo::row(
            repo,
            if rel.is_some() { "pass" } else { "fail" },
            "repo-local skill surface",
            rel,
        ),
    );
}

pub fn standards_enforcement_check(repo: &Path, checks: &mut serde_json::Map<String, Value>) {
    let failures = crate::audit::agent::standards::enforcement::failures_for_paths(
        repo,
        "agent-standards/enforcement.json",
        "agent-standards/enforcement.tsv",
        "agent-standards/enforcement-audit.tsv",
        "scripts/check-agent-standards",
    );
    let ok = failures.is_empty();
    let detail = if ok {
        "standards enforcement surface validates"
    } else {
        "standards enforcement surface invalid"
    };
    checks.insert(
        "standards-enforcement".to_string(),
        crate::target_repo::row(
            repo,
            if ok { "pass" } else { "fail" },
            detail,
            Some("agent-standards/enforcement.json"),
        ),
    );
}

pub fn runtime_check(repo: &Path, checks: &mut serde_json::Map<String, Value>) {
    let runtime = [
        "docs/runtime.md",
        "validation_artifacts/runtime/runtime-legibility.json",
    ]
    .into_iter()
    .find(|rel| repo.join(rel).exists());
    let needed = [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "go.mod",
        "src",
    ]
    .into_iter()
    .any(|rel| repo.join(rel).exists());
    let (status, detail) = if needed && runtime.is_none() {
        (
            "blocked",
            "runnable repo requires runtime legibility receipt",
        )
    } else if runtime.is_some() {
        ("pass", "runtime legibility surface present")
    } else {
        ("not_applicable", "no runnable product signal detected")
    };
    checks.insert(
        "runtime-legibility".to_string(),
        crate::target_repo::row(repo, status, detail, runtime),
    );
}

pub fn check_ids() -> Vec<String> {
    BASELINE_FILES
        .iter()
        .map(|rel| format!("baseline-file:{rel}"))
        .chain(
            BASELINE_DIRS
                .iter()
                .map(|rel| format!("baseline-dir:{rel}")),
        )
        .collect()
}

pub fn directory_listing_digest(path: &Path) -> String {
    let mut files = walkdir::WalkDir::new(path)
        .into_iter()
        .flatten()
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            entry
                .path()
                .strip_prefix(path)
                .ok()
                .map(|rel| rel.to_string_lossy().to_string())
        })
        .collect::<Vec<_>>();
    files.sort();
    digest::bytes(files.join("\n").as_bytes())
}

fn non_placeholder(repo: &Path, rel: &str) -> bool {
    let Ok(text) = crate::target_repo::safe_fs::read_to_string(repo, rel) else {
        return false;
    };
    let lower = text.to_ascii_lowercase();
    text.trim().len() >= 80
        && !["todo", "tbd", "placeholder", "lorem ipsum"]
            .iter()
            .any(|token| lower.contains(token))
}

fn baseline_detail(ok: bool) -> &'static str {
    if ok {
        "present and non-placeholder"
    } else {
        "missing or placeholder"
    }
}
