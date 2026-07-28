use super::super::contracts::{RepositoryPath, Sha256Digest};
use super::super::path;

const MAX_INPUTS: usize = 256;

pub(in crate::generated_authority) fn paths(
    values: Vec<String>,
) -> Result<Vec<RepositoryPath>, &'static str> {
    if values.len() > MAX_INPUTS {
        return Err("generated_authority_input_count_invalid");
    }
    let values = values
        .into_iter()
        .map(path::parse)
        .collect::<Result<Vec<_>, _>>()?;
    path::sorted_unique(&values)
        .then_some(values)
        .ok_or("generated_authority_paths_noncanonical")
}

pub(in crate::generated_authority) fn digest(value: &str) -> Result<Sha256Digest, &'static str> {
    let mut bytes = [0_u8; 32];
    if value.len() != 64 {
        return Err("generated_authority_digest_invalid");
    }
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(chunk).map_err(|_| "generated_authority_digest_invalid")?;
        bytes[index] =
            u8::from_str_radix(text, 16).map_err(|_| "generated_authority_digest_invalid")?;
    }
    if value.bytes().any(|byte| matches!(byte, b'A'..=b'F')) {
        return Err("generated_authority_digest_invalid");
    }
    Ok(Sha256Digest::validated(bytes))
}

pub(in crate::generated_authority) fn command(
    value: &str,
    executable: &str,
) -> Result<(), &'static str> {
    if value.len() > 512
        || value.split_whitespace().next() != Some(executable)
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err("generated_authority_command_invalid");
    }
    Ok(())
}

pub(in crate::generated_authority) fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'+' | b'_' | b'-'))
}

pub(in crate::generated_authority) fn sorted_strings(values: &[String]) -> bool {
    !values.is_empty() && values.windows(2).all(|pair| pair[0] < pair[1])
}

pub(in crate::generated_authority) fn replacement_targets(values: &[String]) -> bool {
    values.len() <= 256
        && sorted_strings(values)
        && values.iter().all(|value| replacement_target(value))
}

fn replacement_target(value: &str) -> bool {
    ["HCT-", "PS-"]
        .iter()
        .any(|prefix| uppercase_semantic_identifier(value, prefix))
        || ["SKILL:", "AGENT:", "COMMAND:", "CONTRACT-REGISTRY:"]
            .iter()
            .any(|prefix| value.strip_prefix(prefix).is_some_and(token))
}

fn uppercase_semantic_identifier(value: &str, prefix: &str) -> bool {
    let Some(suffix) = value.strip_prefix(prefix) else {
        return false;
    };
    let mut bytes = suffix.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        && bytes.all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
}
