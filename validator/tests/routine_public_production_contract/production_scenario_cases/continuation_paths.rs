use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn continuations_root(state_root: &Path) -> PathBuf {
    state_root.join("adapter/continuations")
}

pub(crate) fn checkpoint_path(state_root: &Path) -> PathBuf {
    let legacy = state_root.join("adapter/routine-continuation.json");
    if legacy.exists() {
        return legacy;
    }
    fs::read_dir(continuations_root(state_root))
        .ok()
        .and_then(json_entry)
        .unwrap_or(legacy)
}

pub(crate) fn checkpoint_stage_path(state_root: &Path) -> PathBuf {
    let directory = continuations_root(state_root);
    if let Some(path) = fs::read_dir(&directory).ok().and_then(json_entry) {
        let stem = path.file_stem().and_then(|value| value.to_str()).unwrap();
        return directory.join(format!(".{stem}.next"));
    }
    state_root.join("adapter/.routine-continuation.next")
}

fn json_entry(entries: fs::ReadDir) -> Option<PathBuf> {
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
}
