use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const APPROVED_DESTINATION: &str = ".codex-worktree/env.sh";

pub(crate) struct PolicyState {
    value: Option<Value>,
    policy_path: PathBuf,
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
    pub(crate) fn failures(&self) -> Vec<String> {
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
        if self.destination != PathBuf::from(APPROVED_DESTINATION) {
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
        if !self.secret_leak_free() {
            out.push("openai_key_policy_secret_shape_detected".to_string());
        }
        out
    }

    pub(crate) fn secret_leak_free(&self) -> bool {
        self.value
            .as_ref()
            .is_none_or(|value| !contains_secret_shape(value))
    }

    pub(crate) fn resolution_value(&self) -> Value {
        json!({
            "source": "local_untracked_env_file",
            "destination_path": APPROVED_DESTINATION,
            "policy_path": self.policy_path.to_string_lossy().replace('\\', "/"),
            "destination_gitignored": self.gitignored,
            "env_file_exists": self.env_file.exists,
            "env_file_mode_secure": self.env_file.mode_secure,
            "env_file_declares_openai_api_key": self.env_file.declares_key,
            "secret_value_redacted": true,
            "secret_value_digest_recorded": false,
            "secret_value_length_recorded": false
        })
    }
}

pub(crate) fn load(root: &Path, rel: &Path) -> PolicyState {
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
        policy_path: rel.to_path_buf(),
        destination,
        env_file,
        gitignored,
        load_errors,
    }
}

pub(crate) fn api_key(root: &Path, rel: &Path) -> Result<String, String> {
    let state = load(root, rel);
    let failures = state.failures();
    if !failures.is_empty() {
        return Err("openai_key_policy_not_passing".to_string());
    }
    let path = root.join(&state.destination);
    let text = std::fs::read_to_string(path).map_err(|_| "openai_key_read_failed".to_string())?;
    parse_api_key(&text).ok_or_else(|| "openai_key_not_found".to_string())
}

pub(crate) fn contains_secret_shape(value: &Value) -> bool {
    let text = serde_json::to_string(value).unwrap_or_default();
    text.contains("sk-")
        || text.contains("sk-proj-")
        || text.contains("OPENAI_API_KEY=")
        || text.contains("Authorization:")
        || text.contains("Bearer ")
}

fn parse_api_key(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(value) = trimmed
            .strip_prefix("export OPENAI_API_KEY=")
            .or_else(|| trimmed.strip_prefix("OPENAI_API_KEY="))
        else {
            continue;
        };
        return Some(unquote(value.trim()).to_string());
    }
    None
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|inner| inner.strip_suffix('\''))
        })
        .unwrap_or(value)
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
