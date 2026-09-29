use super::*;

#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

#[cfg(unix)]
pub(crate) fn open_staged_parent(path: &Path) -> Result<(File, CString), String> {
    if !path.is_absolute() {
        return Err("staged publication path is not absolute".to_owned());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "staged publication parent is absent".to_owned())?;
    let name = path
        .file_name()
        .ok_or_else(|| "staged publication leaf is absent".to_owned())?;
    let name = CString::new(name.as_bytes())
        .map_err(|_| "staged publication leaf contains NUL".to_owned())?;
    let mut current =
        File::open("/").map_err(|_| "staged publication root open failed".to_owned())?;
    for component in parent.components() {
        use std::path::Component;
        let Component::Normal(component) = component else {
            if matches!(component, Component::RootDir) {
                continue;
            }
            return Err("staged publication parent is not canonical".to_owned());
        };
        let component = CString::new(component.as_bytes())
            .map_err(|_| "staged publication parent contains NUL".to_owned())?;
        current = open_staged_leaf(
            &current,
            &component,
            0x0000 | DIRECTORY_FLAG | no_follow_nonblock_flags(),
            0,
        )
        .map_err(|_| "staged publication parent open failed closed".to_owned())?;
        if !current
            .metadata()
            .map_err(|_| "staged publication parent metadata unavailable".to_owned())?
            .is_dir()
        {
            return Err("staged publication parent component is not a directory".to_owned());
        }
    }
    Ok((current, name))
}

#[cfg(unix)]
pub(crate) fn verify_staged_publication_path(
    path: &Path,
    expected_parent: &File,
    expected_leaf: &fs::Metadata,
) -> Result<(), String> {
    let (parent, name) = open_staged_parent(path)?;
    let parent_metadata = parent
        .metadata()
        .map_err(|_| "final staged publication parent metadata unavailable".to_owned())?;
    let expected_parent_metadata = expected_parent
        .metadata()
        .map_err(|_| "expected staged publication parent metadata unavailable".to_owned())?;
    if parent_metadata.dev() != expected_parent_metadata.dev()
        || parent_metadata.ino() != expected_parent_metadata.ino()
    {
        return Err("staged publication path parent changed".to_owned());
    }
    let leaf = open_staged_leaf(&parent, &name, 0x0000 | no_follow_nonblock_flags(), 0)
        .map_err(|_| "final staged publication leaf unavailable".to_owned())?;
    let leaf_metadata = leaf
        .metadata()
        .map_err(|_| "final staged publication leaf metadata unavailable".to_owned())?;
    if identity(&leaf_metadata) != identity(expected_leaf) {
        return Err("staged publication path leaf changed".to_owned());
    }
    Ok(())
}

pub(crate) fn stage_manifest_sources_after_cleanup<F>(
    manifest_bytes: &[u8],
    templates: &Path,
    output: &Path,
    after_inspection: F,
) -> Result<Vec<StagedTemplate>, String>
where
    F: FnOnce(),
{
    if manifest_bytes.is_empty() || manifest_bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err("manifest size is outside the bounded contract".to_owned());
    }
    let manifest_paths = parse_authorable_templates(manifest_bytes)?;
    validate_manifest_paths(&manifest_paths)?;
    let observed = inspect_source_tree(templates)?;
    let observed_paths = observed.sources.keys().cloned().collect::<BTreeSet<_>>();
    let manifest_set = manifest_paths.iter().cloned().collect::<BTreeSet<_>>();
    if observed_paths != manifest_set
        || observed
            .directories
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
            != manifest_directory_set(&manifest_paths)
    {
        return Err("manifest and real template source set differ".to_owned());
    }

    after_inspection();
    clean_authority_outputs(output)?;

    let staging = output.join(STAGING_DIRECTORY);
    fs::create_dir_all(&staging).map_err(|_| "staging directory creation failed".to_owned())?;
    let mut total = 0usize;
    let mut rows = Vec::with_capacity(manifest_paths.len());
    for (index, source_path) in manifest_paths.iter().enumerate() {
        let source = observed
            .sources
            .get(source_path)
            .ok_or_else(|| "manifest source disappeared from the inspected set".to_owned())?;
        let bytes = read_stable_source(source)?;
        total = total
            .checked_add(bytes.len())
            .ok_or_else(|| "template source byte total overflowed".to_owned())?;
        if total > MAX_TOTAL_BYTES {
            return Err("template source byte total exceeded its bound".to_owned());
        }
        write_staged(&staging.join(format!("{index:04}.bin")), &bytes)?;
        rows.push(StagedTemplate {
            source_path: source.source_path.clone(),
            target_path: source.target_path.clone(),
            bytes,
            unix_mode: source.identity.mode,
        });
    }
    verify_final_source_tree(templates, &observed)?;
    Ok(rows)
}

pub(crate) fn finish_operation<T>(
    output: &Path,
    result: std::thread::Result<Result<T, String>>,
    panic_error: &'static str,
) -> Result<T, String> {
    match result {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => match clean_authority_outputs(output) {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(cleanup_error),
        },
        Err(_) => match clean_authority_outputs(output) {
            Ok(()) => Err(panic_error.to_owned()),
            Err(cleanup_error) => Err(cleanup_error),
        },
    }
}

pub(crate) fn clean_authority_outputs(output: &Path) -> Result<(), String> {
    let catalog = output.join(GENERATED_CATALOG);
    let staging = output.join(STAGING_DIRECTORY);
    let catalog_result = remove_output(&catalog);
    let staging_result = remove_output(&staging);
    let catalog_absent = matches!(
        fs::symlink_metadata(&catalog),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound
    );
    let staging_absent = matches!(
        fs::symlink_metadata(&staging),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound
    );
    if catalog_result.is_err() || staging_result.is_err() || !catalog_absent || !staging_absent {
        return Err("authority-bearing output cleanup failed".to_owned());
    }
    Ok(())
}

pub(crate) fn remove_output(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path).map_err(|_| "output directory removal failed".to_owned())
        }
        Ok(_) => fs::remove_file(path).map_err(|_| "output object removal failed".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("output object metadata is unavailable".to_owned()),
    }
}

pub(crate) fn manifest_directory_set(paths: &[String]) -> BTreeSet<String> {
    let mut directories = BTreeSet::from(["templates".to_owned()]);
    for path in paths {
        let mut directory = String::new();
        for component in path
            .rsplit_once('/')
            .map_or("", |(parent, _)| parent)
            .split('/')
        {
            if directory.is_empty() {
                directory.push_str(component);
            } else {
                directory.push('/');
                directory.push_str(component);
            }
            directories.insert(directory.clone());
        }
    }
    directories
}

pub(crate) fn verify_final_source_tree(
    templates: &Path,
    expected: &ObservedTree,
) -> Result<(), String> {
    let final_tree = inspect_source_tree(templates)?;
    if final_tree.directories != expected.directories
        || final_tree.sources.len() != expected.sources.len()
        || final_tree.sources.iter().any(|(path, source)| {
            expected.sources.get(path).is_none_or(|original| {
                source.source_path != original.source_path
                    || source.target_path != original.target_path
                    || source.absolute != original.absolute
                    || source.identity != original.identity
            })
        })
    {
        return Err("template source tree changed after inspection".to_owned());
    }
    Ok(())
}

pub(crate) fn read_manifest(manifest: &Path, templates: &Path) -> Result<Vec<u8>, String> {
    #[cfg(not(unix))]
    {
        let _ = (manifest, templates);
        return Err("exact template source classification is unsupported on this host".to_owned());
    }
    #[cfg(unix)]
    {
        let root = fs::symlink_metadata(templates)
            .map_err(|_| "template root metadata is unavailable".to_owned())?;
        if root.file_type().is_symlink() || !root.is_dir() {
            return Err("template root is not one real directory".to_owned());
        }
        let metadata = fs::symlink_metadata(manifest)
            .map_err(|_| "manifest metadata is unavailable".to_owned())?;
        classify_regular(&metadata, root.dev(), MAX_MANIFEST_BYTES)?;
        read_regular_exact(manifest, &identity(&metadata), MAX_MANIFEST_BYTES)
    }
}
