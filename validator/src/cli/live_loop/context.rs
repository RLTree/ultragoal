use super::graph;
use crate::cli::live_loop::LiveLoopCommand;
use serde_json::Value;
use std::path::Path;

pub(crate) struct AuditContext {
    pub(crate) candidate_digest: String,
    pub(crate) cache_mode: String,
    pub(crate) changed_files_digest: String,
    pub(crate) input_digest: String,
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
        Self {
            candidate_digest,
            tier: command.tier.clone(),
            cache_mode: command.cache_mode.clone(),
            changed_files_digest,
            input_digest,
        }
    }

    pub(crate) fn tasks(&self) -> Vec<Box<dyn FnOnce() -> Value + Send + 'static>> {
        graph::tasks(
            &self.candidate_digest,
            &self.changed_files_digest,
            &self.input_digest,
            &self.tier,
            &self.cache_mode,
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

#[cfg(test)]
mod tests {
    use super::verify_cache_hit;

    #[test]
    fn cache_hit_verification_fails_stale_or_wrong_digest() {
        assert_eq!(verify_cache_hit("key-a", "key-a"), "pass");
        assert_eq!(
            verify_cache_hit("key-a", "key-b"),
            "fail_stale_or_wrong_digest_cache_hit"
        );
    }
}
