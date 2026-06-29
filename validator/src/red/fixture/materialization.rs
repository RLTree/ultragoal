use crate::red::fixture::row::result_row;
use serde_json::Value;
use std::path::Path;

pub(crate) struct Observation {
    pub(crate) error: String,
    pub(crate) check: String,
    pub(crate) ok: bool,
}

pub(crate) fn simple_row(root: &Path, packet_rel: &str, expected: &Value, error: &str) -> Value {
    result_row(
        root,
        packet_rel,
        expected,
        error,
        crate::red::fixture::row::expected_status(expected, error),
        None,
        Some(&crate::red::fixture::row::expected_check(expected)),
    )
}

pub(crate) fn base_fixture_json_result(
    result: Result<Value, String>,
    root: &Path,
    packet_rel: &str,
    expected: &Value,
) -> Result<Value, Value> {
    result.map_err(|_| simple_row(root, packet_rel, expected, "base_fixture_malformed_json"))
}

pub(crate) fn base_fixture_error(root: &Path, rel: &str) -> Option<String> {
    if !crate::red::fixture::bases::is_allowed(rel) {
        return Some("invalid_base_fixture_path".to_string());
    }
    if crate::package::inventory::package_path_error(root, rel).is_some() {
        return Some("base_fixture_path_escapes_root".to_string());
    }
    if !root.join(rel).is_file() {
        return Some("base_fixture_missing".to_string());
    }
    None
}
