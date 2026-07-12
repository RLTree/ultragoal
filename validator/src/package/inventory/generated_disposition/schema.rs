use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

const CONTRACT_ID: &str = "harness-ultragoal-successor-contract-v2";
const MAX_INPUTS: usize = 256;
const MAX_REPLACEMENTS: usize = 256;
const MAX_SURFACES: usize = 256;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema_version: String,
    contract_id: String,
    surfaces: Vec<Surface>,
}

#[derive(Clone, Deserialize)]
#[serde(tag = "disposition", rename_all = "snake_case", deny_unknown_fields)]
enum Surface {
    CanonicalProjection {
        output: String,
        generator: String,
        recipe: String,
        inputs: Vec<String>,
    },
    RetainedContext {
        output: String,
        sha256: String,
        reason: String,
        replacement_targets: Vec<String>,
        preserve: bool,
        physical_deletion_authorized: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Row {
    CanonicalProjection {
        generator: String,
        inputs: Vec<String>,
    },
    RetainedContext {
        sha256: String,
        replacement_targets: Vec<String>,
    },
}

pub(super) fn parse(bytes: &[u8]) -> Result<BTreeMap<String, Row>, String> {
    let registry: Registry = serde_json::from_slice(bytes)
        .map_err(|_| "generated disposition registry is invalid JSON".to_string())?;
    if registry.schema_version != "GeneratedSurfaceAuthority-v2" {
        return Err("generated disposition registry schema is unsupported".to_string());
    }
    if registry.contract_id != CONTRACT_ID {
        return Err("generated disposition registry contract ID mismatch".to_string());
    }
    if registry.surfaces.len() > MAX_SURFACES {
        return Err("generated disposition registry exceeds its row limit".to_string());
    }
    let mut rows = BTreeMap::new();
    for surface in registry.surfaces {
        let (output, row) = validate(surface)?;
        if rows.insert(output, row).is_some() {
            return Err("generated disposition registry has duplicate outputs".to_string());
        }
    }
    Ok(rows)
}

fn validate(surface: Surface) -> Result<(String, Row), String> {
    match surface {
        Surface::CanonicalProjection {
            output,
            generator,
            recipe,
            inputs,
        } => {
            validate_output(&output)?;
            if !safe_generator(&generator) || recipe != "input-digest-index-v1" {
                return Err("canonical generated disposition is invalid".to_string());
            }
            validate_sorted_paths(&inputs, MAX_INPUTS)?;
            if inputs.is_empty()
                || inputs
                    .iter()
                    .any(|input| input == &output || input.starts_with(".codex-worktree/"))
            {
                return Err("canonical generated disposition inputs are invalid".to_string());
            }
            Ok((output, Row::CanonicalProjection { generator, inputs }))
        }
        Surface::RetainedContext {
            output,
            sha256,
            reason,
            replacement_targets,
            preserve,
            physical_deletion_authorized,
        } => {
            validate_output(&output)?;
            if !hex_digest(&sha256)
                || reason.is_empty()
                || reason.len() > 1024
                || reason.trim() != reason
                || reason.bytes().any(|byte| byte.is_ascii_control())
                || !preserve
                || physical_deletion_authorized
            {
                return Err("retained generated disposition is invalid".to_string());
            }
            validate_sorted_references(&replacement_targets)?;
            Ok((
                output,
                Row::RetainedContext {
                    sha256,
                    replacement_targets,
                },
            ))
        }
    }
}

fn validate_output(output: &str) -> Result<(), String> {
    if !safe_path(output)
        || !["generated/", "docs/generated/", "examples/generated/"]
            .iter()
            .any(|prefix| output.starts_with(prefix))
    {
        return Err("generated disposition output path is invalid".to_string());
    }
    Ok(())
}

fn validate_sorted_paths(paths: &[String], limit: usize) -> Result<(), String> {
    if paths.len() > limit || paths.iter().any(|path| !safe_path(path)) {
        return Err("generated disposition input path is invalid".to_string());
    }
    sorted_unique(paths, "generated disposition inputs are noncanonical")
}

fn validate_sorted_references(references: &[String]) -> Result<(), String> {
    if references.is_empty()
        || references.len() > MAX_REPLACEMENTS
        || references
            .iter()
            .any(|reference| !safe_reference(reference))
    {
        return Err("generated disposition replacement is invalid".to_string());
    }
    sorted_unique(
        references,
        "generated disposition replacements are noncanonical",
    )
}

fn sorted_unique(values: &[String], message: &str) -> Result<(), String> {
    let mut sorted = values.to_vec();
    sorted.sort();
    if sorted != values || sorted.iter().collect::<BTreeSet<_>>().len() != values.len() {
        return Err(message.to_string());
    }
    Ok(())
}

fn safe_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-'))
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && value
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn safe_generator(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.starts_with("HCT-")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte == b'-')
}

fn upper_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
}

fn lower_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn registry_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn safe_reference(value: &str) -> bool {
    value.strip_prefix("HCT-").is_some_and(upper_token)
        || value.strip_prefix("PS-").is_some_and(upper_token)
        || value.strip_prefix("SKILL:").is_some_and(lower_token)
        || value.strip_prefix("AGENT:").is_some_and(lower_token)
        || value.strip_prefix("COMMAND:").is_some_and(lower_token)
        || value
            .strip_prefix("CONTRACT-REGISTRY:")
            .is_some_and(registry_token)
}

fn hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
