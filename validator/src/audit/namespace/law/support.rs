use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn mixed_domain_folder(components: &[&str]) -> bool {
    components.iter().any(|part| {
        let lowered = part.to_ascii_lowercase();
        let tokens = lowered.split(['-', '_']).collect::<BTreeSet<_>>();
        tokens.contains("runtime") && tokens.contains("product")
            || tokens.contains("schema") && tokens.contains("ui")
            || tokens.contains("review") && tokens.contains("install")
    })
}

pub(super) fn root_route_allowed(rel: &str) -> bool {
    matches!(
        rel,
        "README.md"
            | "REPORT.md"
            | "Cargo.toml"
            | "Cargo.lock"
            | "rust-toolchain.toml"
            | "plugin-manifest-draft.json"
            | ".gitignore"
    )
}

pub(crate) fn str_field(row: &Value, key: &str) -> String {
    row.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
