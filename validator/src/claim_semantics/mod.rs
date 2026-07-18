pub(crate) mod lane;
pub(crate) mod ready;

pub(crate) mod coverage;

use crate::digest;
use crate::json_boundary;
use serde_json::Value;

pub(crate) fn str_field(value: &Value, key: &str) -> String {
    json_boundary::string(value, key).unwrap_or_default()
}

pub(crate) fn bool_field(value: &Value, key: &str) -> bool {
    json_boundary::bool_value(value, key).unwrap_or(false)
}

pub(crate) fn array_strings(value: &Value, key: &str) -> Vec<String> {
    json_boundary::string_array(value, key)
}

pub(crate) fn path_contains(parent: &str, child: &str) -> bool {
    lane::paths::contains(parent, child).unwrap_or(false)
}

pub(crate) fn canonical_digest(value: &Value) -> Result<String, String> {
    Ok(digest::canonical_json(value))
}
