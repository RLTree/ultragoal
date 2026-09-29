//! Unsupported executable languages must never receive a Rust-semantic green.
pub(crate) fn executable_source(path: &str, bytes: &[u8]) -> bool {
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str());
    matches!(
        extension,
        Some(
            "py" | "pyi"
                | "js"
                | "jsx"
                | "ts"
                | "tsx"
                | "mjs"
                | "cjs"
                | "sh"
                | "bash"
                | "zsh"
                | "fish"
                | "go"
                | "java"
                | "kt"
                | "swift"
                | "c"
                | "h"
                | "cc"
                | "cpp"
                | "hpp"
                | "cs"
                | "rb"
                | "php"
                | "pl"
                | "lua"
                | "ex"
                | "exs"
                | "clj"
                | "scala"
                | "dart"
                | "vue"
                | "svelte"
                | "wasm"
                | "html"
                | "htm"
        )
    ) || bytes.starts_with(b"#!") && !path.ends_with(".rs")
}
