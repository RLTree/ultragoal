use crate::cli::observe::types::ObserveOperation;
use serde_json::{Value, json};

pub(crate) fn candidate_digest_failure(body: &str, expected: &str) -> Option<String> {
    typed_candidate_digests(body)
        .into_iter()
        .find(|digest| digest != expected)
        .map(|digest| format!("observability_query_candidate_digest_mismatch:{digest}!={expected}"))
}

fn typed_candidate_digests(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        collect_candidate_digests(&value, &mut out);
    }
    out
}

fn collect_candidate_digests(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                if key == "candidate_digest" {
                    collect_candidate_string(value, out);
                } else {
                    collect_candidate_digests(value, out);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_candidate_digests(item, out);
            }
        }
        Value::String(raw) if starts_with_json(raw) => {
            if let Ok(nested) = serde_json::from_str::<Value>(raw) {
                collect_candidate_digests(&nested, out);
            }
        }
        _ => {}
    }
}

fn starts_with_json(raw: &str) -> bool {
    raw.trim_start()
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch, '{' | '['))
}

fn collect_candidate_string(value: &Value, out: &mut Vec<String>) {
    let Some(candidate) = value.as_str().filter(|candidate| valid_digest(candidate)) else {
        return;
    };
    if !out.iter().any(|item| item == candidate) {
        out.push(candidate.to_string());
    }
}

fn valid_digest(candidate: &str) -> bool {
    candidate.len() == "sha256:".len() + 64
        && candidate.starts_with("sha256:")
        && candidate["sha256:".len()..]
            .chars()
            .all(|ch| ch.is_ascii_hexdigit())
}

pub(crate) fn bounded_rows(body: String, byte_limit: usize) -> Vec<Value> {
    let redacted = redact_private_paths(&body);
    let clipped = if redacted.len() > byte_limit {
        format!("{}...[truncated]", &redacted[..byte_limit])
    } else {
        redacted
    };
    vec![json!({"body": clipped})]
}

fn redact_private_paths(body: &str) -> String {
    let home_redacted = redact_path_marker(body, private_home_marker(), "[redacted-home-path]");
    redact_path_marker(
        &home_redacted,
        private_tmp_marker(),
        "[redacted-private-tmp-path]",
    )
}

fn private_home_marker() -> &'static str {
    concat!("/", "Users/")
}

fn private_tmp_marker() -> &'static str {
    concat!("/", "private", "/tmp/")
}

fn redact_path_marker(body: &str, marker: &str, replacement: &str) -> String {
    let mut output = String::with_capacity(body.len());
    let mut index = 0;
    while let Some(offset) = body[index..].find(marker) {
        let start = index + offset;
        output.push_str(&body[index..start]);
        let end = body[start..]
            .char_indices()
            .find_map(|(idx, ch)| path_delimiter(ch).then_some(start + idx))
            .unwrap_or(body.len());
        output.push_str(replacement);
        index = end;
    }
    output.push_str(&body[index..]);
    output
}

fn path_delimiter(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '"' | '\'' | ',' | '}' | ']' | ')')
}

pub(crate) fn has_matches(body: &str, operation: ObserveOperation) -> bool {
    let body = body.trim();
    if body.is_empty() {
        return false;
    }
    match operation {
        ObserveOperation::MetricsQuery => !body.contains("\"result\":[]"),
        ObserveOperation::TracesQuery => trace_total(body) > 0,
        _ => true,
    }
}

fn trace_total(body: &str) -> i64 {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value.get("total").and_then(Value::as_i64).or_else(|| {
                value
                    .get("data")
                    .and_then(Value::as_array)
                    .map(|items| items.len() as i64)
            })
        })
        .unwrap_or(0)
}
