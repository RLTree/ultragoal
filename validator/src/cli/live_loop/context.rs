use super::graph;
use crate::cli::live_loop::LiveLoopCommand;
use serde_json::Value;
use std::path::Path;

pub(crate) struct AuditContext {
    pub(crate) candidate_digest: String,
    pub(crate) cache_mode: String,
    pub(crate) changed_files_digest: String,
    pub(crate) input_digest: String,
    package_digest_baseline_ms: Option<u64>,
    tier: String,
}

impl AuditContext {
    pub(crate) fn new(root: &Path, candidate_digest: String, command: &LiveLoopCommand) -> Self {
        let changed_files = changed_files(root);
        let changed_files_digest = crate::digest::bytes(changed_files.join("\n").as_bytes());
        let input_digest = crate::digest::bytes(
            format!(
                "{}:{}:{}:{}",
                candidate_digest, command.tier, command.cache_mode, changed_files_digest
            )
            .as_bytes(),
        );
        let package_digest_baseline_ms = package_digest_baseline_ms(root, &candidate_digest);
        Self {
            candidate_digest,
            tier: command.tier.clone(),
            cache_mode: command.cache_mode.clone(),
            changed_files_digest,
            input_digest,
            package_digest_baseline_ms,
        }
    }

    pub(crate) fn tasks(&self) -> Vec<Box<dyn FnOnce() -> Value + Send + 'static>> {
        graph::tasks(
            &self.candidate_digest,
            &self.changed_files_digest,
            &self.input_digest,
            &self.tier,
            &self.cache_mode,
            self.package_digest_baseline_ms,
        )
    }
}

pub(crate) fn verify_cache_hit(expected_key: &str, observed_key: &str) -> &'static str {
    if expected_key == observed_key {
        "pass"
    } else {
        "fail_stale_or_wrong_digest_cache_hit"
    }
}

fn changed_files(root: &Path) -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["status", "--short", "--untracked-files=all"])
        .current_dir(root)
        .output();
    output
        .ok()
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn package_digest_baseline_ms(root: &Path, candidate_digest: &str) -> Option<u64> {
    let receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/package-digest.json"),
    )
    .ok()?;
    let receipt_candidate = receipt.get("candidate_digest")?.as_str()?;
    if receipt_candidate != candidate_digest {
        return None;
    }
    receipt
        .pointer("/event/duration_ms")
        .and_then(serde_json::Value::as_u64)
}

#[cfg(test)]
mod tests {
    use super::{package_digest_baseline_ms, verify_cache_hit};
    use serde_json::json;

    #[test]
    fn cache_hit_verification_fails_stale_or_wrong_digest() {
        assert_eq!(verify_cache_hit("key-a", "key-a"), "pass");
        assert_eq!(
            verify_cache_hit("key-a", "key-b"),
            "fail_stale_or_wrong_digest_cache_hit"
        );
    }

    #[test]
    fn package_digest_baseline_uses_same_candidate_telemetry_only() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
            "live-loop-package-baseline",
        );
        let receipt_path = root.join("validation_artifacts/observability/package-digest.json");
        crate::json_boundary::write_json(
            &receipt_path,
            &json!({
                "candidate_digest": "sha256:current",
                "event": {"duration_ms": 123}
            }),
        )
        .expect("package digest telemetry receipt");

        assert_eq!(
            package_digest_baseline_ms(&root, "sha256:current"),
            Some(123)
        );
        assert_eq!(package_digest_baseline_ms(&root, "sha256:stale"), None);
        crate::json_boundary::write_json(
            &receipt_path,
            &json!({
                "candidate_digest": 7,
                "event": {"duration_ms": 123}
            }),
        )
        .expect("malformed package digest telemetry receipt");
        assert_eq!(package_digest_baseline_ms(&root, "sha256:current"), None);
    }
}
