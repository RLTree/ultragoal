use super::limits::{MAX_ATTRIBUTE_KEY_BYTES, MAX_ATTRIBUTE_VALUE_BYTES, MAX_IDENTIFIER_BYTES};

const REDACTED: &str = "[REDACTED]";

pub(super) fn validate_identifier(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > MAX_IDENTIFIER_BYTES {
        return Err(format!(
            "observe-invalid-{label}: identifier length is outside the bound"
        ));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
    {
        return Err(format!(
            "observe-invalid-{label}: identifier syntax is not supported"
        ));
    }
    if sensitive_value(value) {
        return Err(format!(
            "observe-invalid-{label}: raw sensitive identifiers are not supported"
        ));
    }
    Ok(())
}

pub(super) fn validate_attribute_key(key: &str) -> Result<(), String> {
    if key.is_empty() || key.len() > MAX_ATTRIBUTE_KEY_BYTES {
        return Err("observe-invalid-attribute-key: key length is outside the bound".to_owned());
    }
    if !key
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return Err("observe-invalid-attribute-key: key syntax is not supported".to_owned());
    }
    Ok(())
}

pub(super) fn is_claim_authority_key(key: &str) -> bool {
    let normalized = normalized_ascii(key);
    [
        "claimstate",
        "claimstatus",
        "claimceiling",
        "supportedclaims",
        "readiness",
        "ready",
        "release",
        "completion",
        "goalcomplete",
        "requirementstate",
    ]
    .iter()
    .any(|reserved| normalized == *reserved || normalized.starts_with(reserved))
}

pub(super) fn sanitize_attribute(key: &str, value: &str) -> Result<Option<String>, String> {
    validate_attribute_key(key)?;
    if value.len() > MAX_ATTRIBUTE_VALUE_BYTES {
        return Err("observe-attribute-limit: value exceeds the byte bound".to_owned());
    }
    if is_claim_authority_key(key) {
        return Err(
            "observe-claim-authority-denied: semantic events cannot carry claim state".to_owned(),
        );
    }
    if sensitive_key(key) || sensitive_value(value) {
        return Ok(None);
    }
    if value.bytes().any(|byte| byte.is_ascii_control()) {
        return Err("observe-invalid-attribute-value: control bytes are not supported".to_owned());
    }
    Ok(Some(value.to_owned()))
}

pub(super) fn contains_sensitive_text(value: &str) -> bool {
    sensitive_value(value) || value.contains(REDACTED) && value != REDACTED
}

fn sensitive_key(key: &str) -> bool {
    let normalized = normalized_ascii(key);
    [
        "secret",
        "password",
        "passwd",
        "token",
        "apikey",
        "authorization",
        "credential",
        "cookie",
        "sessionid",
        "email",
        "phone",
        "fullname",
        "username",
        "owner",
        "person",
        "displayname",
        "account",
        "filepath",
        "absolutepath",
        "homedir",
        "path",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn sensitive_value(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let compact = normalized_ascii(value);
    lower.contains("bearer ")
        || lower.contains("basic ")
        || lower.contains("/users/")
        || lower.contains("/home/")
        || lower.contains("\\users\\")
        || lower.contains("token=")
        || lower.contains("token%3d")
        || lower.contains("password=")
        || lower.starts_with("sk-")
        || lower.starts_with("ghp_")
        || lower.starts_with("github_pat_")
        || lower.starts_with("xoxb-")
        || lower.starts_with("xoxp-")
        || lower.starts_with("akia")
        || lower.starts_with("~/")
        || lower.starts_with('/')
        || looks_like_windows_path(value)
        || compact.contains("authorizationbearer")
        || looks_like_email(value)
        || looks_like_jwt(value)
        || looks_like_phone(value)
        || looks_like_ssn(value)
}

fn looks_like_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.contains(char::is_whitespace)
}

fn looks_like_jwt(value: &str) -> bool {
    let mut parts = value.split('.');
    let Some(first) = parts.next() else {
        return false;
    };
    let Some(second) = parts.next() else {
        return false;
    };
    let Some(third) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && [first, second, third]
            .iter()
            .all(|part| part.len() >= 8 && part.bytes().all(is_base64_url))
}

fn is_base64_url(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'=')
}

fn looks_like_phone(value: &str) -> bool {
    let digits = value.bytes().filter(u8::is_ascii_digit).count();
    digits >= 10
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || b"+-. ()".contains(&byte))
}

fn looks_like_windows_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}

fn looks_like_ssn(value: &str) -> bool {
    let digits = value.bytes().filter(u8::is_ascii_digit).count();
    digits == 9
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'-' || byte == b' ')
}

fn normalized_ascii(value: &str) -> String {
    value
        .bytes()
        .filter(u8::is_ascii_alphanumeric)
        .map(|byte| byte.to_ascii_lowercase() as char)
        .collect()
}
