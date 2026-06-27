use crate::digest;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::Path;

pub const REQUIRED_MARKERS: &[&str] = &[
    "baseline-files",
    "execplan-dirs",
    "proof-root",
    "coverage-enforcement",
    "standards-enforcement",
];

pub struct GateResult {
    pub command: Value,
    pub markers: Vec<String>,
    pub check: Value,
}

pub fn run(repo: &Path) -> GateResult {
    let start = crate::audit::clock::now_iso();
    let Some(gate) = gate_path(repo) else {
        let command = json!({"id": "target-repo-check", "command": "scripts/check", "cwd": repo.to_string_lossy(), "exit": 127, "started_at": start, "completed_at": crate::audit::clock::now_iso()});
        return GateResult {
            command,
            markers: Vec::new(),
            check: crate::target_repo::row(repo, "fail", "scripts/check missing", None),
        };
    };
    let rel = gate_rel(repo, &gate);
    let script = crate::target_repo::safe_fs::read_to_string(repo, &rel).unwrap_or_default();
    let unsafe_lines = unsafe_lines(&script);
    let markers = markers(&script);
    let stdout = marker_stdout(&markers);
    let stderr = unsafe_lines.join("\n").into_bytes();
    let exit = if unsafe_lines.is_empty() { 0 } else { 1 };
    let marker_ok = REQUIRED_MARKERS
        .iter()
        .all(|required| markers.contains(*required));
    let substantive_ok = substantive_script(repo, &script);
    let status = if exit == 0 && marker_ok && substantive_ok {
        "pass"
    } else {
        "fail"
    };
    let detail = format!(
        "scripts/check statically declared required markers: {}; substantive_checks={substantive_ok}",
        markers.iter().cloned().collect::<Vec<_>>().join(","),
    );
    let command = json!({
        "id": "target-repo-check",
        "command": format!("static-read {rel}"),
        "cwd": repo.to_string_lossy(),
        "exit": exit,
        "started_at": start,
        "completed_at": crate::audit::clock::now_iso(),
        "stdout": {"path": "<inline:stdout>", "digest": digest::bytes(&stdout)},
        "stderr": {"path": "<inline:stderr>", "digest": digest::bytes(&stderr)}
    });
    GateResult {
        command,
        markers: markers.into_iter().collect(),
        check: crate::target_repo::row(repo, status, &detail, Some(&rel)),
    }
}

fn gate_path(repo: &Path) -> Option<std::path::PathBuf> {
    ["scripts/check", "scripts/check.sh"]
        .into_iter()
        .map(|rel| repo.join(rel))
        .find(|path| {
            path.strip_prefix(repo)
                .ok()
                .and_then(|rel| rel.to_str())
                .is_some_and(|rel| crate::target_repo::safe_fs::is_regular_file(repo, rel))
        })
}

pub(crate) fn gate_rel(repo: &Path, gate: &Path) -> String {
    gate.strip_prefix(repo)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "scripts/check".to_string())
}

fn markers(script: &str) -> BTreeSet<String> {
    script
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("echo \"harness-check:")
                .and_then(|rest| rest.strip_suffix('"'))
                .and_then(|rest| rest.strip_suffix(" pass"))
        })
        .map(ToOwned::to_owned)
        .collect()
}

fn marker_stdout(markers: &BTreeSet<String>) -> Vec<u8> {
    markers
        .iter()
        .map(|marker| format!("harness-check:{marker} pass"))
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes()
}

fn substantive_script(repo: &Path, text: &str) -> bool {
    let assertions = text
        .lines()
        .map(str::trim)
        .filter_map(test_assertion)
        .collect::<Vec<_>>();
    let distinct = assertions
        .iter()
        .map(|(kind, rel)| format!("{kind}:{rel}"))
        .collect::<BTreeSet<_>>();
    distinct.len() >= 10
        && assertions
            .iter()
            .all(|(kind, rel)| assertion_matches(repo, kind, rel))
}

fn test_assertion(line: &str) -> Option<(&str, &str)> {
    line.strip_prefix("test -f ")
        .map(|rel| ("file", rel))
        .or_else(|| line.strip_prefix("test -d ").map(|rel| ("dir", rel)))
}

fn assertion_matches(repo: &Path, kind: &str, rel: &str) -> bool {
    match kind {
        "file" => crate::target_repo::safe_fs::is_regular_file(repo, rel),
        "dir" => crate::target_repo::safe_fs::is_dir(repo, rel),
        _ => false,
    }
}

fn unsafe_lines(script: &str) -> Vec<String> {
    script
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !allowed_line(line))
        .map(|line| format!("unsupported gate line: {line}"))
        .collect()
}

fn allowed_line(line: &str) -> bool {
    line == "#!/usr/bin/env bash"
        || line == "set -euo pipefail"
        || allowed_test(line)
        || allowed_marker(line)
}

fn allowed_test(line: &str) -> bool {
    test_assertion(line).is_some_and(|(_, path)| {
        !path.starts_with('/')
            && !path.contains("..")
            && path
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || "/._-".contains(ch))
    })
}

fn allowed_marker(line: &str) -> bool {
    line.strip_prefix("echo \"harness-check:")
        .and_then(|rest| rest.strip_suffix('"'))
        .and_then(|rest| rest.strip_suffix(" pass"))
        .is_some_and(|marker| {
            !marker.is_empty()
                && marker
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        })
}

#[cfg(test)]
mod tests {
    #[test]
    fn assertion_matches_rejects_unknown_assertion_kind() {
        let root =
            crate::self_tests::boundaries::support::temp_root("target-check-gate-unknown-kind");
        std::fs::create_dir_all(&root).expect("target check gate root");
        std::fs::write(root.join("file.txt"), "ok").expect("file");
        assert!(!super::assertion_matches(&root, "socket", "file.txt"));
        assert_eq!(
            super::gate_rel(
                std::path::Path::new("/repo"),
                std::path::Path::new("/outside/check")
            ),
            "scripts/check"
        );
        std::fs::remove_dir_all(root).expect("cleanup target check gate");
    }
}
