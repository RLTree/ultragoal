fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_stable_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.contains("../")
        && !value.contains("/..")
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
        && value
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn safe_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PATH_BYTES
        && !value.chars().any(char::is_control)
        && !value.contains("../")
        && !value.contains("/..")
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PATH_BYTES
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.chars().any(char::is_control)
        && value
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn normalized(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}
