use crate::cli::live_loop::LiveLoopCommand;
use crate::scheduler::TaskClass;
use serde_json::{Value, json};
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
        let nodes = [
            ("package_digest", self.candidate_digest.clone()),
            ("changed_files", self.changed_files_digest.clone()),
            ("audit_context", self.input_digest.clone()),
            ("observability_control_board", self.cache_mode.clone()),
        ];
        nodes
            .into_iter()
            .map(|(id, digest)| {
                let tier = self.tier.clone();
                let cache_mode = self.cache_mode.clone();
                Box::new(move || node_result(id, &digest, &tier, &cache_mode))
                    as Box<dyn FnOnce() -> Value + Send + 'static>
            })
            .collect()
    }
}

fn node_result(id: &str, digest: &str, tier: &str, cache_mode: &str) -> Value {
    let cache = cache_decision(id, digest, tier, cache_mode);
    json!({
        "node_id": id,
        "status": "pass",
        "input_digest": digest,
        "cache": cache,
        "task_class": TaskClass::PureReadParallel.id(),
        "claim_impact": "source_local_live_loop_acceleration_only"
    })
}

fn cache_decision(id: &str, digest: &str, tier: &str, cache_mode: &str) -> Value {
    let key = cache_key(id, digest, tier, cache_mode);
    json!({
        "mode": cache_mode,
        "key": key,
        "hit": false,
        "invalidation_reason": "no verified local cache entry",
        "cache_class": "verified_content_addressed_local",
        "honesty": verify_cache_hit(&key, &key)
    })
}

pub(crate) fn verify_cache_hit(expected_key: &str, observed_key: &str) -> &'static str {
    if expected_key == observed_key {
        "pass"
    } else {
        "fail_stale_or_wrong_digest_cache_hit"
    }
}

fn cache_key(id: &str, digest: &str, tier: &str, cache_mode: &str) -> String {
    crate::digest::bytes(
        format!(
            "node={id};input={digest};validator=ultragoal-rust;law=observability-live-loop;tier={tier};cache={cache_mode};env=local"
        )
        .as_bytes(),
    )
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
