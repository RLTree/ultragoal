use super::contracts::RedCatalogError;
use std::fs;
use std::path::Path;

pub(super) struct RedPacketSource {
    pub(super) relative: String,
    pub(super) bytes: Vec<u8>,
}

pub(super) fn packet_sources(root: &Path) -> Result<Vec<RedPacketSource>, RedCatalogError> {
    let directory = root.join("fixtures/red");
    let metadata = fs::symlink_metadata(&directory)
        .map_err(|_| error("red_catalog_fixture_directory_unreadable", "fixtures/red"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(error(
            "red_catalog_fixture_directory_not_regular",
            "fixtures/red",
        ));
    }
    let entries = fs::read_dir(&directory)
        .map_err(|_| error("red_catalog_fixture_directory_unreadable", "fixtures/red"))?;
    let mut paths = entries
        .map(|entry| {
            entry
                .map(|value| value.path())
                .map_err(|_| error("red_catalog_fixture_entry_unreadable", "fixtures/red"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    paths
        .into_iter()
        .map(|path| packet_source(root, &path))
        .collect()
}

pub(super) fn catalog_bytes(root: &Path) -> Result<Vec<u8>, RedCatalogError> {
    regular_bytes(root, "templates/RED_FIXTURES.json")
}

fn packet_source(root: &Path, path: &Path) -> Result<RedPacketSource, RedCatalogError> {
    let relative = path
        .strip_prefix(root)
        .ok()
        .and_then(Path::to_str)
        .map(|value| value.replace('\\', "/"))
        .ok_or_else(|| {
            error(
                "red_catalog_fixture_path_invalid",
                &path.display().to_string(),
            )
        })?;
    if !relative.starts_with("fixtures/red/") || !relative.ends_with(".json") {
        return Err(error("red_catalog_fixture_path_unknown", &relative));
    }
    regular_bytes(root, &relative).map(|bytes| RedPacketSource { relative, bytes })
}

fn regular_bytes(root: &Path, relative: &str) -> Result<Vec<u8>, RedCatalogError> {
    let path = root.join(relative);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| error("red_catalog_regular_file_unreadable", relative))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(error("red_catalog_regular_file_required", relative));
    }
    crate::digest::read_file_bytes(&path)
        .map_err(|_| error("red_catalog_regular_file_unreadable", relative))
}

fn error(code: &'static str, path: &str) -> RedCatalogError {
    RedCatalogError::new(code, Some(path.to_string()))
}
