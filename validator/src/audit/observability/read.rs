use serde_json::Value;
use std::path::Path;

pub(super) fn json(root: &Path, rel: &str) -> Value {
    crate::json_boundary::read_json(&root.join(rel)).unwrap_or(Value::Null)
}
