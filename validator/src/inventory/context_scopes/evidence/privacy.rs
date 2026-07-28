use serde_json::Value;

const PRIVATE_PATH_MARKERS: &str = "/users/|/home/|/root/|/tmp/|/private/|\\users\\|file://";
const SECRET_PREFIXES: &str = "sk-|ghp_|gho_|github_pat_|xoxb-|xoxp-|akia";

pub(super) fn safe_nested_strings(value: &Value, parent_key: Option<&str>) -> bool {
    if parent_key.is_some_and(private_identifier_or_secret_key) {
        return false;
    }
    match value {
        Value::String(text) => !private_path_or_secret(text),
        Value::Array(values) => values
            .iter()
            .all(|value| safe_nested_strings(value, parent_key)),
        Value::Object(values) => values.iter().all(|(key, value)| {
            !private_path_or_secret(key) && safe_nested_strings(value, Some(key))
        }),
        _ => true,
    }
}

fn private_identifier_or_secret_key(key: &str) -> bool {
    let normalized = key
        .bytes()
        .filter(u8::is_ascii_alphanumeric)
        .map(|byte| byte.to_ascii_lowercase() as char)
        .collect::<String>();
    normalized.contains("sessionid")
        || normalized.contains("taskid")
        || [
            "secret",
            "password",
            "passwd",
            "token",
            "apikey",
            "authorization",
            "credential",
            "cookie",
            "privatekey",
        ]
        .iter()
        .any(|private| normalized == *private || normalized.ends_with(private))
}

fn private_path_or_secret(value: &str) -> bool {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    let public_network_value = lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("wss://");
    (!public_network_value
        && PRIVATE_PATH_MARKERS
            .split('|')
            .any(|needle| lower.contains(needle)))
        || lower.starts_with("~/")
        || lower.starts_with("bearer ")
        || lower.starts_with("basic ")
        || lower.contains("password=")
        || lower.contains("passwd=")
        || lower.contains("token=")
        || lower.contains("api_key=")
        || lower.contains("apikey=")
        || lower.contains("secret=")
        || lower.contains("authorization:")
        || SECRET_PREFIXES
            .split('|')
            .any(|prefix| lower.starts_with(prefix))
        || public_network_value
            && lower.split_once("://").is_some_and(|(_, tail)| {
                tail.split('/')
                    .next()
                    .is_some_and(|authority| authority.contains('@') && authority.contains(':'))
            })
        || trimmed.contains("-----BEGIN") && trimmed.contains("PRIVATE KEY-----")
        || trimmed.as_bytes().get(1) == Some(&b':')
            && trimmed
                .as_bytes()
                .get(2)
                .is_some_and(|byte| matches!(byte, b'\\' | b'/'))
}
