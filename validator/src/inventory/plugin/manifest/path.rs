use crate::context::ReadSession;
use std::path::{Component, Path};

const MAX_ASSET_BYTES: u64 = 32 * 1024 * 1024;
const MAX_COMPANION_BYTES: u64 = 4 * 1024 * 1024;

pub(super) fn relative(value: &str) -> Option<&str> {
    let relative = value.strip_prefix("./")?;
    (!relative.is_empty()
        && value.len() <= 512
        && !Path::new(relative).is_absolute()
        && Path::new(relative)
            .components()
            .all(|part| matches!(part, Component::Normal(_))))
    .then_some(relative)
}

pub(super) fn matches(value: &str, expected: &str) -> bool {
    relative(value).is_some_and(|value| value.trim_end_matches('/') == expected)
}

pub(super) fn asset_exists(reads: &ReadSession, root: &Path, value: &str, png_only: bool) -> bool {
    if !relative(value).is_some_and(|value| value.starts_with("assets/")) {
        return false;
    }
    if png_only && !value.ends_with(".png") {
        return false;
    }
    reads
        .read_bounded(&root.join(value), MAX_ASSET_BYTES)
        .is_ok()
}

pub(super) fn companion_exists(reads: &ReadSession, root: &Path, value: &str) -> bool {
    let Some(relative) = relative(value) else {
        return false;
    };
    let path = root.join(relative);
    let Ok(metadata) = std::fs::symlink_metadata(&path) else {
        return false;
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return false;
    }
    let Ok(root) = root.canonicalize() else {
        return false;
    };
    path.canonicalize()
        .is_ok_and(|resolved| resolved.starts_with(&root))
        && reads.read_bounded(&path, MAX_COMPANION_BYTES).is_ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn manifest_paths_require_a_confined_dot_slash_prefix() {
        for invalid in [
            "",
            "skills/",
            "/skills/",
            "./",
            "./../skills/",
            "./skills/../other/",
            ".//skills/",
        ] {
            assert!(!super::matches(invalid, "skills"), "{invalid}");
        }
        assert!(super::matches("./skills/", "skills"));
    }
}
