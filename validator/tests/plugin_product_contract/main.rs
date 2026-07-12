#![allow(clippy::expect_used)]

mod dependency_closure;
mod route_contract;
mod source_contract;
mod zero_write;

use std::path::PathBuf;

pub fn root() -> PathBuf {
    let root = std::env::var_os("HUL_REPO_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("current directory"));
    assert!(root.join(".codex-plugin/plugin.json").is_file());
    root
}

pub fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|error| {
        panic!("failed to read {path}: {error}");
    })
}
