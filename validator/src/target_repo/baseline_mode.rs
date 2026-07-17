use super::baseline::BASELINE_FILES;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub fn mode_checks(repo: &Path, mode: &str, checks: &mut serde_json::Map<String, Value>) {
    let extras = fresh_extra_files(repo);
    if mode == "init" {
        fresh_init_checks(repo, extras, checks);
    } else {
        retrofit_checks(repo, checks);
    }
}

fn fresh_init_checks(
    repo: &Path,
    extras: Vec<String>,
    checks: &mut serde_json::Map<String, Value>,
) {
    let detail = if extras.is_empty() {
        "fresh init has no non-harness preexisting files".to_string()
    } else {
        format!(
            "existing repo signals require retrofit mode: {}",
            extras.into_iter().take(5).collect::<Vec<_>>().join(",")
        )
    };
    checks.insert(
        "mode-provenance".to_string(),
        crate::target_repo::row(
            repo,
            if detail.starts_with("fresh") {
                "pass"
            } else {
                "fail"
            },
            &detail,
            None,
        ),
    );
    checks.insert(
        "retrofit-backlog".to_string(),
        crate::target_repo::row(repo, "not_applicable", "fresh init mode", None),
    );
}

fn retrofit_checks(repo: &Path, checks: &mut serde_json::Map<String, Value>) {
    let blocker = blocker_path(repo);
    checks.insert(
        "mode-provenance".to_string(),
        crate::target_repo::row(repo, "pass", "retrofit mode selected", None),
    );
    checks.insert(
        "retrofit-backlog".to_string(),
        crate::target_repo::row(
            repo,
            if blocker.is_some() { "pass" } else { "fail" },
            "retrofit blockers/backlog are explicit",
            blocker.as_deref(),
        ),
    );
}

fn blocker_path(repo: &Path) -> Option<String> {
    [
        "validation_artifacts/harness-engineering/target-repo-blockers.json",
        "validation_artifacts/harness-engineering/retrofit-backlog.json",
        "VERIFICATION_BACKLOG.json",
    ]
    .into_iter()
    .find(|rel| repo.join(rel).is_file())
    .map(ToOwned::to_owned)
}

fn fresh_extra_files(repo: &Path) -> Vec<String> {
    let allowed = BASELINE_FILES
        .iter()
        .chain(
            [
                "docs/observability.md",
                "docs/plugin-cohesion-manifest.json",
                "docs/product-cohesion.md",
                "scripts/check",
                "scripts/check.sh",
                "schemas/product-fitness-receipt.schema.json",
                "scripts/observe",
                "scripts/observe.sh",
            ]
            .iter(),
        )
        .copied()
        .collect::<BTreeSet<_>>();
    let prefixes = [
        "validation_artifacts/",
        "agent-standards/",
        ".agents/skills/",
        ".agents/plugins/",
        ".codex/skills/",
    ];
    let mut extras = walkdir::WalkDir::new(repo)
        .into_iter()
        .flatten()
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            entry
                .path()
                .strip_prefix(repo)
                .ok()
                .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        })
        .filter(|rel| {
            !allowed.contains(rel.as_str())
                && !rel.ends_with("/.keep")
                && !rel.ends_with(".keep")
                && !prefixes.iter().any(|prefix| rel.starts_with(prefix))
        })
        .collect::<Vec<_>>();
    extras.sort();
    extras
}
