use serde::Serialize;
use serde_json::Value;
use std::path::Path;

pub fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = crate::digest::read_file_bytes(path)
        .map_err(|err| format!("{}: json read failed: {err}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|err| format!("{}: malformed json: {err}", path.display()))
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|err| format!("{}: json encode failed: {err}", path.display()))?;
    crate::output_path::write(path, format!("{text}\n"), "json")
}

pub fn object_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.as_object().and_then(|obj| obj.get(key))
}

pub fn string(value: &Value, key: &str) -> Option<String> {
    object_get(value, key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

pub fn bool_value(value: &Value, key: &str) -> Option<bool> {
    object_get(value, key).and_then(Value::as_bool)
}

pub fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    object_get(value, key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

pub fn string_array(value: &Value, key: &str) -> Vec<String> {
    array(value, key)
        .into_iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(all(test, unix))]
mod tests {
    use super::read_json;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn read_json_rejects_symlink_leaf() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ultragoal-json-test-{stamp}"));
        fs::create_dir(&dir).expect("create temp dir");
        let real = dir.join("manifest.json");
        let link = dir.join("plugin-manifest-draft.json");
        fs::write(&real, br#"{"ok":true}"#).expect("write real json");
        symlink(&real, &link).expect("create symlink");

        assert!(read_json(&real).is_ok());
        assert!(read_json(&link).is_err());

        fs::remove_dir_all(&dir).expect("remove temp dir");
    }
}
