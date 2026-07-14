use std::path::{Component, Path, PathBuf};

pub(super) fn source_bases(relative: &str) -> (PathBuf, PathBuf) {
    let path = Path::new(relative);
    let directory = path.parent().unwrap_or_else(|| Path::new(""));
    let file = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let module = if matches!(file, "lib.rs" | "main.rs" | "mod.rs") {
        directory.to_path_buf()
    } else {
        directory.join(file.trim_end_matches(".rs"))
    };
    (directory.to_path_buf(), module)
}

pub(super) fn conventional_candidates(base: &Path, module: &str) -> Vec<String> {
    [
        base.join(format!("{module}.rs")),
        base.join(module).join("mod.rs"),
    ]
    .into_iter()
    .filter_map(normalize)
    .collect()
}

pub(super) fn normalize(path: PathBuf) -> Option<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => parts.push(value.to_str()?.to_string()),
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    let relative = parts.join("/");
    relative.starts_with("validator/src/").then_some(relative)
}
