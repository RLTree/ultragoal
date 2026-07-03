pub(super) fn raw_authority_marker(text: &str) -> Option<&'static str> {
    if text.contains("serde_json::Map")
        || text.contains("BTreeMap<String, Value")
        || text.contains("HashMap<String, Value")
        || text.contains("raw_map")
    {
        return Some("raw_map");
    }
    if contains_raw_path_authority(text) {
        return Some("raw_path");
    }
    if text.contains("raw_string") {
        return Some("raw_string");
    }
    if super::contains_json_value_binding(text) {
        return Some("raw_json");
    }
    if text.contains("\"raw_") || text.contains("raw_observation") {
        return Some("raw_observation");
    }
    None
}

fn contains_raw_path_authority(text: &str) -> bool {
    let compact = text.replace(['\n', '\t'], " ");
    compact.contains("raw_path")
        || compact.contains(": PathBuf")
        || compact.contains("-> PathBuf")
        || compact.contains("PathBuf::from(")
        || compact.contains(": &Path")
        || compact.contains("std::path::Path")
}
