use super::*;

pub(crate) fn command_observation_receipt_path(
    root: &Path,
    row: &serde_json::Value,
) -> Option<PathBuf> {
    let relative = Path::new(row.get("command_observation_receipt")?.as_str()?);
    (!relative.is_absolute()
        && !relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir)))
    .then(|| root.join(relative))
}

pub(crate) trait WithValue {
    fn with_value(self, key: &str, value: serde_json::Value) -> Self;
    fn without_key(self, key: &str) -> Self;
}

impl WithValue for serde_json::Value {
    fn with_value(mut self, key: &str, value: serde_json::Value) -> Self {
        self.as_object_mut()
            .expect("timing row object")
            .insert(key.to_string(), value);
        self
    }

    fn without_key(mut self, key: &str) -> Self {
        self.as_object_mut().expect("timing row object").remove(key);
        self
    }
}
