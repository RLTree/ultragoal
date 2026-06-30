use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) struct Observation {
    pub(super) prompt_input_digest: String,
    pub(super) output_digest: String,
    pub(super) request_id: String,
    pub(super) prompt_tokens: i64,
    pub(super) completion_tokens: i64,
    pub(super) total_tokens: i64,
    pub(super) latency_ms: i64,
    pub(super) rate_limit_observed: bool,
    pub(super) failures: Vec<String>,
}

pub(super) fn execute(
    root: &Path,
    command: &super::call::CallCommand,
    budget: &super::budget::BudgetSelection,
) -> Result<Observation, String> {
    let mut failures = Vec::new();
    let key = match super::policy::api_key(root, Path::new(super::config::DEFAULT_POLICY)) {
        Ok(key) => key,
        Err(err) => {
            failures.push(err);
            String::new()
        }
    };
    let body = request_body(&command.model_identity);
    let prompt_input_digest = crate::digest::canonical_json(&body);
    if command.input_digest != crate::digest::ZERO && command.input_digest != prompt_input_digest {
        failures.push("openai_live_prompt_input_digest_mismatch".to_string());
    }
    let paths = TempPaths::new();
    let body_text = body.to_string();
    write_temp(&paths.request, body_text.as_bytes(), &mut failures);
    let output = if failures.is_empty() {
        run_curl(&key, &paths, timeout_seconds(budget), &mut failures)
    } else {
        None
    };
    let response = read_temp(&paths.response, &mut failures);
    let headers = read_temp(&paths.headers, &mut failures);
    paths.cleanup();
    let output_digest = crate::digest::bytes(response.as_bytes());
    let parsed = serde_json::from_str::<Value>(&response).unwrap_or(Value::Null);
    failures.extend(http_failures(output.as_deref(), &parsed));
    Ok(Observation {
        prompt_input_digest,
        output_digest,
        request_id: request_id(&headers, &parsed),
        prompt_tokens: usage(&parsed, "input_tokens"),
        completion_tokens: usage(&parsed, "output_tokens"),
        total_tokens: usage(&parsed, "total_tokens"),
        latency_ms: latency_ms(output.as_deref()),
        rate_limit_observed: headers
            .lines()
            .any(|line| line.to_ascii_lowercase().starts_with("x-ratelimit-")),
        failures,
    })
}

fn request_body(model: &str) -> Value {
    json!({
        "model": model,
        "input": "Return exactly {\"ok\":true} for a Harness Ultragoal live-provider boundary probe.",
        "max_output_tokens": 16,
        "store": false
    })
}

fn run_curl(
    key: &str,
    paths: &TempPaths,
    timeout_seconds: i64,
    failures: &mut Vec<String>,
) -> Option<String> {
    let mut child = match Command::new("/usr/bin/curl")
        .arg("-K")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            failures.push("openai_live_curl_spawn_failed".to_string());
            return None;
        }
    };
    if let Some(stdin) = child.stdin.as_mut() {
        if stdin
            .write_all(config(key, paths, timeout_seconds).as_bytes())
            .is_err()
        {
            failures.push("openai_live_curl_config_write_failed".to_string());
        }
    }
    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(_) => {
            failures.push("openai_live_curl_wait_failed".to_string());
            return None;
        }
    };
    if !output.status.success() {
        failures.push("openai_live_curl_exit_failed".to_string());
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

fn config(key: &str, paths: &TempPaths, timeout_seconds: i64) -> String {
    format!(
        concat!(
            "url = \"https://api.openai.com/v1/responses\"\n",
            "request = \"POST\"\n",
            "header = \"Authorization: Bearer {}\"\n",
            "header = \"Content-Type: application/json\"\n",
            "data = \"@{}\"\n",
            "output = \"{}\"\n",
            "dump-header = \"{}\"\n",
            "silent\nshow-error\nmax-time = {}\n",
            "write-out = \"%{{http_code}} %{{time_total}}\"\n"
        ),
        key,
        paths.request.display(),
        paths.response.display(),
        paths.headers.display(),
        timeout_seconds
    )
}

fn http_failures(output: Option<&str>, parsed: &Value) -> Vec<String> {
    let mut failures = Vec::new();
    let status = output
        .and_then(|text| text.split_whitespace().next())
        .unwrap_or("000");
    if !status.starts_with('2') {
        failures.push(format!("openai_live_http_status_not_success:{status}"));
    }
    if parsed.get("id").and_then(Value::as_str).is_none() {
        failures.push("openai_live_response_missing_id".to_string());
    }
    failures
}

fn timeout_seconds(budget: &super::budget::BudgetSelection) -> i64 {
    budget
        .selected
        .get("timeout_ms")
        .and_then(Value::as_i64)
        .unwrap_or(30000)
        / 1000
}

fn request_id(headers: &str, parsed: &Value) -> String {
    headers
        .lines()
        .find_map(|line| {
            line.strip_prefix("x-request-id:")
                .or_else(|| line.strip_prefix("X-Request-Id:"))
                .map(|value| value.trim().to_string())
        })
        .or_else(|| parsed.get("id").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_else(|| "not_available:openai_live".to_string())
}

fn usage(parsed: &Value, key: &str) -> i64 {
    parsed
        .get("usage")
        .and_then(|usage| usage.get(key))
        .and_then(Value::as_i64)
        .unwrap_or(0)
}

fn latency_ms(output: Option<&str>) -> i64 {
    output
        .and_then(|text| text.split_whitespace().nth(1))
        .and_then(|seconds| seconds.parse::<f64>().ok())
        .map(|seconds| (seconds * 1000.0).round() as i64)
        .unwrap_or(0)
}

fn write_temp(path: &Path, bytes: &[u8], failures: &mut Vec<String>) {
    if std::fs::write(path, bytes).is_err() {
        failures.push("openai_live_temp_write_failed".to_string());
    }
}

fn read_temp(path: &Path, failures: &mut Vec<String>) -> String {
    match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => {
            failures.push("openai_live_temp_read_failed".to_string());
            String::new()
        }
    }
}

struct TempPaths {
    request: PathBuf,
    response: PathBuf,
    headers: PathBuf,
}

impl TempPaths {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or(0);
        let base = std::env::temp_dir().join(format!("ultragoal-openai-live-{stamp}"));
        Self {
            request: base.with_extension("request.json"),
            response: base.with_extension("response.json"),
            headers: base.with_extension("headers.txt"),
        }
    }

    fn cleanup(&self) {
        let _ = std::fs::remove_file(&self.request);
        let _ = std::fs::remove_file(&self.response);
        let _ = std::fs::remove_file(&self.headers);
    }
}
