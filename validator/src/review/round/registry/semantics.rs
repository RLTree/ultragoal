use serde::Deserialize;
use serde_json::Value;

pub(super) const SCHEMA_PATH: &str = "schemas/codex-registry-exposure.schema.json";
const SCHEMA_SHA256: &str =
    "sha256:12e4a2e9a97cd3fe90ee092b9d839cf9c72bf249f05fe5d0bc24c764934ea844";
const SCHEMA_ID: &str =
    "https://harness-ultragoal.local/schemas/codex-registry-exposure.schema.json";

pub(super) fn schema_errors(bytes: &[u8], schema: &Value, exposure: &Value) -> Vec<String> {
    if crate::digest::bytes(bytes) != SCHEMA_SHA256
        || schema.get("$id").and_then(Value::as_str) != Some(SCHEMA_ID)
    {
        return vec!["authoritative registry schema mismatch".to_owned()];
    }
    crate::schema_catalog::bound_schema_errors(
        "codex-registry-exposure.schema.json",
        schema,
        exposure,
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentManifest {
    name: String,
    description: String,
    developer_instructions: String,
    sandbox_mode: String,
}

pub(super) fn manifest_is_current(bytes: &[u8], expected_name: &str) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let Ok(manifest) = toml::from_str::<AgentManifest>(text) else {
        return false;
    };
    manifest.name == expected_name
        && valid_text(&manifest.description, 1024, false)
        && valid_text(&manifest.developer_instructions, 16 * 1024, true)
        && manifest.sandbox_mode == "read-only"
}

fn valid_text(value: &str, maximum: usize, multiline: bool) -> bool {
    !value.trim().is_empty()
        && value.len() <= maximum
        && !value.chars().any(|character| {
            character.is_control() && !(multiline && matches!(character, '\n' | '\r' | '\t'))
        })
}
