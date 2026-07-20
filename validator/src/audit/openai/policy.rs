use serde_json::Value;
use std::path::{Path, PathBuf};

const APPROVED_DESTINATION: &str = ".codex-worktree/env.sh";

pub(super) struct PolicyState {
    value: Option<Value>,
    destination: PathBuf,
    env_file: EnvFileState,
    gitignored: bool,
    load_errors: Vec<String>,
}

struct EnvFileState {
    exists: bool,
    mode_secure: bool,
    declares_key: bool,
    malformed: bool,
}

impl PolicyState {
    pub(super) fn failures(&self) -> Vec<String> {
        let mut out = self.load_errors.clone();
        let Some(value) = &self.value else {
            out.push("openai_key_policy_missing_or_malformed".to_string());
            return out;
        };
        require_string(
            value,
            "schema",
            "harness-ultragoal.openai-key-policy.v1",
            &mut out,
        );
        require_string(value, "law_id", super::LAW_ID, &mut out);
        require_string(value, "env_var", "OPENAI_API_KEY", &mut out);
        require_string(value, "active_destination", APPROVED_DESTINATION, &mut out);
        require_string(value, "secret_serialization_policy", "forbid", &mut out);
        require_string(
            value,
            "model_output_authority",
            "observation_only",
            &mut out,
        );
        if self.destination != std::path::Path::new(APPROVED_DESTINATION) {
            out.push("openai_key_policy_unapproved_destination".to_string());
        }
        if !self.gitignored {
            out.push("openai_key_destination_not_gitignored".to_string());
        }
        if !self.env_file.exists {
            out.push("openai_key_destination_missing".to_string());
        }
        if !self.env_file.mode_secure {
            out.push("openai_key_destination_mode_not_0600".to_string());
        }
        if !self.env_file.declares_key {
            out.push("openai_key_destination_missing_openai_api_key".to_string());
        }
        if self.env_file.malformed {
            out.push("openai_key_destination_malformed".to_string());
        }
        if self.value.as_ref().is_some_and(contains_secret_shape) {
            out.push("openai_key_policy_secret_shape_detected".to_string());
        }
        out
    }
}

pub(super) fn load(root: &Path, rel: &Path) -> PolicyState {
    let mut load_errors = Vec::new();
    let value = match crate::json_boundary::read_json(&root.join(rel)) {
        Ok(value) => Some(value),
        Err(err) => {
            load_errors.push(format!("openai_key_policy_load_failed:{err}"));
            None
        }
    };
    let destination = value
        .as_ref()
        .and_then(|policy| policy.get("active_destination"))
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(APPROVED_DESTINATION));
    let env_file = inspect_env_file(&root.join(&destination));
    let gitignored = destination_is_ignored(root, &destination);
    PolicyState {
        value,
        destination,
        env_file,
        gitignored,
        load_errors,
    }
}

pub(crate) fn contains_secret_shape(value: &Value) -> bool {
    let text = serde_json::to_string(value).unwrap_or_default();
    text.contains("sk-")
        || text.contains("sk-proj-")
        || text.contains("OPENAI_API_KEY=")
        || text.contains("Authorization:")
        || text.contains("Bearer ")
}

fn inspect_env_file(path: &Path) -> EnvFileState {
    let exists = path.is_file();
    let mode_secure = secure_mode(path);
    let mut declares_key = false;
    let mut malformed = false;
    if exists {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                for line in text.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("export OPENAI_API_KEY=")
                        || trimmed.starts_with("OPENAI_API_KEY=")
                    {
                        declares_key = true;
                    }
                    if trimmed.contains("sk-") && !declares_key {
                        malformed = true;
                    }
                }
            }
            Err(_) => malformed = true,
        }
    }
    EnvFileState {
        exists,
        mode_secure,
        declares_key,
        malformed,
    }
}

#[cfg(unix)]
fn secure_mode(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|meta| meta.permissions().mode() & 0o777 == 0o600)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn secure_mode(path: &Path) -> bool {
    path.is_file()
}

fn destination_is_ignored(root: &Path, destination: &Path) -> bool {
    let first = destination
        .components()
        .next()
        .map(|part| part.as_os_str().to_string_lossy().to_string())
        .unwrap_or_default();
    let Ok(text) = std::fs::read_to_string(root.join(".gitignore")) else {
        return false;
    };
    text.lines()
        .map(str::trim)
        .any(|line| line == format!("{first}/") || line == first)
}

fn require_string(value: &Value, key: &str, expected: &str, failures: &mut Vec<String>) {
    if value.get(key).and_then(Value::as_str) != Some(expected) {
        failures.push(format!("openai_key_policy_field_mismatch:{key}"));
    }
}
