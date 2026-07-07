pub(super) fn path_affects_surface(path: &str, surface_id: &str) -> bool {
    match surface_id {
        "fmt_check" => is_rust_source(path) || is_rust_format_config(path),
        "build_check" | "live_loop_measurement_rust_tests" => {
            is_rust_source(path) || is_rust_build_input(path)
        }
        "line_caps_check" => is_rust_source(path),
        "namespace_check" => {
            is_rust_source(path)
                || path == "docs/namespace-law-exceptions.json"
                || path == "plugin-manifest-draft.json"
        }
        "schema_validation" => is_json_surface(path) || path.starts_with("schemas/"),
        "package_inventory" => true,
        _ => true,
    }
}

fn is_rust_source(path: &str) -> bool {
    path.starts_with("validator/src/") && path.ends_with(".rs")
}

fn is_rust_format_config(path: &str) -> bool {
    matches!(path, "rustfmt.toml" | ".rustfmt.toml")
}

fn is_rust_build_input(path: &str) -> bool {
    matches!(path, "Cargo.toml" | "Cargo.lock" | "validator/Cargo.toml")
}

fn is_json_surface(path: &str) -> bool {
    path.ends_with(".json") || path.ends_with(".jsonl")
}
