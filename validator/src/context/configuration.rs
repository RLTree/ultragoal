use super::error::ContextError;
use super::types::{ConfigurationIdentity, SecretSourceIdentity};
use std::collections::BTreeMap;

const PUBLIC_CONFIGURATION_KEYS: &[&str] = &[
    "claim",
    "contract_id",
    "feature_set",
    "mode",
    "output_format",
    "plugin_version",
    "profile",
    "ultragoal.adopted_handoff_manifest_sha256",
];

fn secret_like(value: &str) -> bool {
    let lowercase = value.to_ascii_lowercase();
    lowercase.starts_with("sk-")
        || lowercase.starts_with("ghp_")
        || lowercase.starts_with("bearer ")
        || lowercase.contains("private_key")
        || lowercase.contains("password=")
        || lowercase.contains("token=")
}

fn public_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|character| character.is_ascii_graphic() || character == ' ')
        && !secret_like(value)
}

pub(super) fn identity(
    values: &BTreeMap<String, String>,
    secret_sources: &BTreeMap<String, String>,
) -> Result<ConfigurationIdentity, ContextError> {
    for (key, value) in values {
        if !PUBLIC_CONFIGURATION_KEYS.contains(&key.as_str()) || !public_value(value) {
            return Err(ContextError::InvalidRequest(format!(
                "configuration {key:?} is not an allowlisted non-secret public value"
            )));
        }
    }
    let mut sources = Vec::with_capacity(secret_sources.len());
    for (name, public_version) in secret_sources {
        let valid_name = !name.is_empty()
            && name.len() <= 64
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte));
        if !valid_name || !public_value(public_version) {
            return Err(ContextError::InvalidRequest(format!(
                "secret source identity {name:?} is not a safe public identifier"
            )));
        }
        sources.push(SecretSourceIdentity {
            name: name.clone(),
            public_version: public_version.clone(),
        });
    }
    Ok(ConfigurationIdentity {
        public_values: values.clone(),
        secret_sources: sources,
    })
}
