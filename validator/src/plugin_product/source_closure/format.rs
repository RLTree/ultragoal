use super::model::{BuildClosureRow, ClosureError};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub(super) fn dep_info_tokens(value: &str) -> Result<Vec<PathBuf>, ClosureError> {
    let flattened = value.replace("\\\n", " ");
    let primary_rule = flattened.split("\n\n").next().unwrap_or_default();
    let Some((_, dependencies)) = primary_rule.split_once(':') else {
        return Err(ClosureError::InvalidDepInfo);
    };
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for character in dependencies.chars() {
        if escaped {
            current.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character.is_whitespace() {
            if !current.is_empty() {
                tokens.push(PathBuf::from(std::mem::take(&mut current)));
            }
        } else {
            current.push(character);
        }
    }
    if escaped {
        return Err(ClosureError::InvalidDepInfo);
    }
    if !current.is_empty() {
        tokens.push(PathBuf::from(current));
    }
    if tokens.is_empty() {
        return Err(ClosureError::InvalidDepInfo);
    }
    Ok(tokens)
}

pub(super) fn aggregate(rows: &[BuildClosureRow]) -> String {
    let mut rows = rows.to_vec();
    rows.sort_by(|left, right| left.path.cmp(&right.path));
    let mut bytes = Vec::new();
    for row in rows {
        bytes.extend_from_slice(row.path.as_bytes());
        bytes.push(b'\t');
        bytes.extend_from_slice(format!("{:?}", row.kind).to_ascii_lowercase().as_bytes());
        bytes.push(b'\t');
        bytes.extend_from_slice(row.sha256.as_bytes());
        bytes.push(b'\t');
        bytes.extend_from_slice(row.byte_length.to_string().as_bytes());
        bytes.push(b'\n');
    }
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(super) fn validate_digest(value: &str) -> Result<(), ClosureError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(ClosureError::InvalidDigest);
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ClosureError::InvalidDigest);
    }
    Ok(())
}
