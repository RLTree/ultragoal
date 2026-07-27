use serde_json::Value;
use std::path::Path;

mod class;
mod law;
mod path_label_edges;
mod raw_string_edges;
mod semantic_names;
mod topology;
mod wire_contract;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn write_text(path: &Path, value: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, value).expect("write text");
}

fn contains(items: &[String], needle: &str) -> bool {
    items.iter().any(|item| item.contains(needle))
}
