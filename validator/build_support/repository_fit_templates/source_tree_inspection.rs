use super::*;

pub(crate) fn inspect_source_tree(templates: &Path) -> Result<ObservedTree, String> {
    #[cfg(not(unix))]
    {
        let _ = templates;
        return Err("exact template source classification is unsupported on this host".to_owned());
    }
    #[cfg(unix)]
    {
        let root = fs::symlink_metadata(templates)
            .map_err(|_| "template root metadata is unavailable".to_owned())?;
        if root.file_type().is_symlink() || !root.is_dir() {
            return Err("template root is not one real directory".to_owned());
        }
        let mut tree = ObservedTree {
            directories: BTreeMap::new(),
            sources: BTreeMap::new(),
        };
        inspect_directory(templates, templates, root.dev(), &mut tree)?;
        if tree.sources.is_empty()
            || tree.sources.len() > MAX_ROWS
            || tree.directories.len() > MAX_ROWS
        {
            return Err("template source count is outside the bounded contract".to_owned());
        }
        Ok(tree)
    }
}

#[cfg(unix)]
pub(crate) fn inspect_directory(
    root: &Path,
    directory: &Path,
    root_device: u64,
    tree: &mut ObservedTree,
) -> Result<(), String> {
    let directory_before = fs::symlink_metadata(directory)
        .map_err(|_| "template directory metadata is unavailable".to_owned())?;
    if directory_before.file_type().is_symlink()
        || !directory_before.is_dir()
        || directory_before.dev() != root_device
    {
        return Err("template directory is linked, substituted, or cross-device".to_owned());
    }
    let mut entries = fs::read_dir(directory)
        .map_err(|_| "template directory enumeration failed".to_owned())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "template directory entry could not be read".to_owned())?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let absolute = entry.path();
        let metadata = fs::symlink_metadata(&absolute)
            .map_err(|_| "template source metadata is unavailable".to_owned())?;
        if metadata.file_type().is_symlink() || metadata.dev() != root_device {
            return Err("template source is linked or cross-device".to_owned());
        }
        if metadata.is_dir() {
            inspect_directory(root, &absolute, root_device, tree)?;
            continue;
        }
        classify_regular(&metadata, root_device, MAX_SOURCE_BYTES)?;
        let relative = absolute
            .strip_prefix(root)
            .map_err(|_| "template source escaped its root".to_owned())?
            .to_str()
            .ok_or_else(|| "template source path is not UTF-8".to_owned())?;
        validate_relative_path(relative)?;
        let source_path = format!("templates/{relative}");
        let row = ObservedSource {
            source_path: source_path.clone(),
            target_path: relative.to_owned(),
            absolute,
            identity: identity(&metadata),
        };
        if tree.sources.insert(source_path, row).is_some() {
            return Err("duplicate template source path".to_owned());
        }
    }
    let directory_after = fs::symlink_metadata(directory)
        .map_err(|_| "template directory final metadata is unavailable".to_owned())?;
    if identity(&directory_after) != identity(&directory_before) {
        return Err("template directory mutated during enumeration".to_owned());
    }
    let relative = directory
        .strip_prefix(root)
        .map_err(|_| "template directory escaped its root".to_owned())?
        .to_str()
        .ok_or_else(|| "template directory path is not UTF-8".to_owned())?;
    if !relative.is_empty() {
        validate_relative_path(relative)?;
    }
    let directory_path = if relative.is_empty() {
        "templates".to_owned()
    } else {
        format!("templates/{relative}")
    };
    if tree
        .directories
        .insert(directory_path, identity(&directory_before))
        .is_some()
    {
        return Err("duplicate template directory path".to_owned());
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn classify_regular(
    metadata: &fs::Metadata,
    root_device: u64,
    maximum_bytes: u64,
) -> Result<(), String> {
    let file_type = metadata.file_type();
    if !metadata.is_file()
        || file_type.is_symlink()
        || file_type.is_fifo()
        || file_type.is_socket()
        || file_type.is_block_device()
        || file_type.is_char_device()
        || metadata.nlink() != 1
        || metadata.dev() != root_device
        || metadata.len() > maximum_bytes
        || !matches!(metadata.permissions().mode() & 0o7777, 0o644 | 0o755)
    {
        return Err("template source is not one bounded single-link regular file".to_owned());
    }
    Ok(())
}

pub(crate) fn read_stable_source(source: &ObservedSource) -> Result<Vec<u8>, String> {
    read_regular_exact(&source.absolute, &source.identity, MAX_SOURCE_BYTES)
}

#[cfg(unix)]
pub(crate) fn read_regular_exact(
    path: &Path,
    expected: &SourceIdentity,
    maximum_bytes: u64,
) -> Result<Vec<u8>, String> {
    let mut options = OpenOptions::new();
    options.read(true).custom_flags(no_follow_nonblock_flags());
    let mut file = options
        .open(path)
        .map_err(|_| "template source open failed closed".to_owned())?;
    let opened = file
        .metadata()
        .map_err(|_| "opened template metadata is unavailable".to_owned())?;
    classify_regular(&opened, expected.device, maximum_bytes)?;
    if identity(&opened) != *expected {
        return Err("template source changed after inspection".to_owned());
    }
    let first = bounded_read(&mut file, maximum_bytes)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|_| "template source rewind failed".to_owned())?;
    let second = bounded_read(&mut file, maximum_bytes)?;
    let handle_after = file
        .metadata()
        .map_err(|_| "template source final handle metadata is unavailable".to_owned())?;
    let path_after = fs::symlink_metadata(path)
        .map_err(|_| "template source final path metadata is unavailable".to_owned())?;
    if first != second
        || first.len() as u64 != expected.length
        || identity(&handle_after) != *expected
        || identity(&path_after) != *expected
    {
        return Err("template source mutated during staging".to_owned());
    }
    Ok(first)
}

#[cfg(not(unix))]
pub(crate) fn read_regular_exact(
    _path: &Path,
    _expected: &SourceIdentity,
    _maximum_bytes: u64,
) -> Result<Vec<u8>, String> {
    Err("exact template source classification is unsupported on this host".to_owned())
}

pub(crate) fn bounded_read(file: &mut File, maximum_bytes: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    file.take(maximum_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "template source read failed".to_owned())?;
    if bytes.len() as u64 > maximum_bytes {
        return Err("template source exceeded its byte bound".to_owned());
    }
    Ok(bytes)
}
