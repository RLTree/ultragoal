//! Std-only build-time classification and staging for repository-fit sources.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};

const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SOURCE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;
const MAX_ROWS: usize = 512;
const GENERATED_CATALOG: &str = "repository_fit_template_catalog.rs";
const STAGING_DIRECTORY: &str = "repository_fit_template_sources";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagedTemplate {
    pub source_path: String,
    pub target_path: String,
    pub bytes: Vec<u8>,
    pub unix_mode: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceIdentity {
    device: u64,
    inode: u64,
    links: u64,
    length: u64,
    mode: u32,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[derive(Clone, Debug)]
struct ObservedSource {
    source_path: String,
    target_path: String,
    absolute: PathBuf,
    identity: SourceIdentity,
}

#[derive(Clone, Debug)]
struct ObservedTree {
    directories: BTreeMap<String, SourceIdentity>,
    sources: BTreeMap<String, ObservedSource>,
}

pub fn generate(manifest: &Path, templates: &Path, output: &Path) -> Result<(), String> {
    generate_with_hooks(manifest, templates, output, || {}, || {}, || {})
}

pub fn generate_with_hooks<F, G, H>(
    manifest: &Path,
    templates: &Path,
    output: &Path,
    after_inspection: F,
    before_manifest_write: G,
    before_catalog_emission: H,
) -> Result<(), String>
where
    F: FnOnce(),
    G: FnOnce(),
    H: FnOnce(),
{
    clean_authority_outputs(output)?;
    let result = catch_unwind(AssertUnwindSafe(|| {
        generate_after_cleanup(
            manifest,
            templates,
            output,
            after_inspection,
            before_manifest_write,
            before_catalog_emission,
        )
    }));
    finish_operation(output, result, "template catalog generation panicked")
}

fn generate_after_cleanup<F, G, H>(
    manifest: &Path,
    templates: &Path,
    output: &Path,
    after_inspection: F,
    before_manifest_write: G,
    before_catalog_emission: H,
) -> Result<(), String>
where
    F: FnOnce(),
    G: FnOnce(),
    H: FnOnce(),
{
    let generated_catalog = output.join(GENERATED_CATALOG);
    let manifest_bytes = read_manifest(manifest, templates)?;
    let rows =
        stage_manifest_sources_with_hook(&manifest_bytes, templates, output, after_inspection)?;
    before_manifest_write();
    let staged_manifest = output.join(STAGING_DIRECTORY).join("manifest.json");
    write_staged(&staged_manifest, &manifest_bytes)?;
    let mut generated = String::new();
    generated.push_str("const MANIFEST_BYTES: &[u8] = include_bytes!(");
    generated.push_str(&format!("{:?}", staged_manifest.to_string_lossy()));
    generated.push_str(");\n");
    generated.push_str("pub(super) static CANONICAL_TEMPLATES: &[TemplateCatalogRow] = &[\n");
    for (index, row) in rows.iter().enumerate() {
        let staged = output
            .join(STAGING_DIRECTORY)
            .join(format!("{index:04}.bin"));
        generated.push_str("    TemplateCatalogRow::regular(");
        generated.push_str(&format!(
            "{:?}, {:?}, include_bytes!({:?}), 0o{:o}",
            row.source_path,
            row.target_path,
            staged.to_string_lossy(),
            row.unix_mode,
        ));
        generated.push_str("),\n");
    }
    generated.push_str("];\n");
    before_catalog_emission();
    write_staged(&generated_catalog, generated.as_bytes())
}

pub fn stage_manifest_sources(
    manifest_bytes: &[u8],
    templates: &Path,
    output: &Path,
) -> Result<Vec<StagedTemplate>, String> {
    stage_manifest_sources_with_hook(manifest_bytes, templates, output, || {})
}

pub fn stage_manifest_sources_with_hook<F>(
    manifest_bytes: &[u8],
    templates: &Path,
    output: &Path,
    after_inspection: F,
) -> Result<Vec<StagedTemplate>, String>
where
    F: FnOnce(),
{
    clean_authority_outputs(output)?;
    let result = catch_unwind(AssertUnwindSafe(|| {
        stage_manifest_sources_after_cleanup(manifest_bytes, templates, output, after_inspection)
    }));
    finish_operation(output, result, "template source staging panicked")
}

fn stage_manifest_sources_after_cleanup<F>(
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

fn finish_operation<T>(
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

fn clean_authority_outputs(output: &Path) -> Result<(), String> {
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

fn remove_output(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path).map_err(|_| "output directory removal failed".to_owned())
        }
        Ok(_) => fs::remove_file(path).map_err(|_| "output object removal failed".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("output object metadata is unavailable".to_owned()),
    }
}

fn manifest_directory_set(paths: &[String]) -> BTreeSet<String> {
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

fn verify_final_source_tree(templates: &Path, expected: &ObservedTree) -> Result<(), String> {
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

fn read_manifest(manifest: &Path, templates: &Path) -> Result<Vec<u8>, String> {
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

fn inspect_source_tree(templates: &Path) -> Result<ObservedTree, String> {
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
fn inspect_directory(
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
fn classify_regular(
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

fn read_stable_source(source: &ObservedSource) -> Result<Vec<u8>, String> {
    read_regular_exact(&source.absolute, &source.identity, MAX_SOURCE_BYTES)
}

#[cfg(unix)]
fn read_regular_exact(
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
fn read_regular_exact(
    _path: &Path,
    _expected: &SourceIdentity,
    _maximum_bytes: u64,
) -> Result<Vec<u8>, String> {
    Err("exact template source classification is unsupported on this host".to_owned())
}

fn bounded_read(file: &mut File, maximum_bytes: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    file.take(maximum_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "template source read failed".to_owned())?;
    if bytes.len() as u64 > maximum_bytes {
        return Err("template source exceeded its byte bound".to_owned());
    }
    Ok(bytes)
}

fn write_staged(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| "staged parent creation failed".to_owned())?;
    }
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    let mut file = options
        .open(path)
        .map_err(|_| "staged source creation failed".to_owned())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| "staged source write failed".to_owned())?;
    let staged = fs::read(path).map_err(|_| "staged source verification failed".to_owned())?;
    if staged != bytes {
        return Err("staged source bytes differ".to_owned());
    }
    Ok(())
}

fn parse_authorable_templates(bytes: &[u8]) -> Result<Vec<String>, String> {
    let key = b"\"authorable_templates\"";
    let matches = bytes
        .windows(key.len())
        .filter(|window| *window == key)
        .count();
    if matches != 1 {
        return Err("manifest must contain one authorable_templates key".to_owned());
    }
    let key_start = bytes
        .windows(key.len())
        .position(|window| window == key)
        .ok_or_else(|| "manifest source list is absent".to_owned())?;
    let mut index = key_start + key.len();
    skip_whitespace(bytes, &mut index);
    expect(bytes, &mut index, b':')?;
    skip_whitespace(bytes, &mut index);
    expect(bytes, &mut index, b'[')?;
    let mut values = Vec::new();
    loop {
        skip_whitespace(bytes, &mut index);
        if bytes.get(index) == Some(&b']') {
            index += 1;
            break;
        }
        values.push(parse_string(bytes, &mut index)?);
        if values.len() > MAX_ROWS {
            return Err("manifest source count exceeded its bound".to_owned());
        }
        skip_whitespace(bytes, &mut index);
        match bytes.get(index) {
            Some(b',') => index += 1,
            Some(b']') => {
                index += 1;
                break;
            }
            _ => return Err("manifest source list is malformed".to_owned()),
        }
    }
    let _ = index;
    Ok(values)
}

fn parse_string(bytes: &[u8], index: &mut usize) -> Result<String, String> {
    expect(bytes, index, b'"')?;
    let mut output = Vec::new();
    while let Some(byte) = bytes.get(*index).copied() {
        *index += 1;
        match byte {
            b'"' => {
                return String::from_utf8(output)
                    .map_err(|_| "manifest source path is not UTF-8".to_owned());
            }
            b'\\' => {
                let escaped = bytes
                    .get(*index)
                    .copied()
                    .ok_or_else(|| "manifest string escape is truncated".to_owned())?;
                *index += 1;
                output.push(match escaped {
                    b'"' | b'\\' | b'/' => escaped,
                    b'b' => 8,
                    b'f' => 12,
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    _ => return Err("manifest source path uses an unsupported escape".to_owned()),
                });
            }
            0..=31 => return Err("manifest source path contains a control byte".to_owned()),
            _ => output.push(byte),
        }
    }
    Err("manifest source string is unterminated".to_owned())
}

fn validate_manifest_paths(paths: &[String]) -> Result<(), String> {
    if paths.is_empty() || paths.len() > MAX_ROWS {
        return Err("manifest source count is outside the bounded contract".to_owned());
    }
    let mut exact = BTreeSet::new();
    let mut folded = BTreeSet::new();
    let mut previous = None;
    for path in paths {
        let target = path
            .strip_prefix("templates/")
            .ok_or_else(|| "manifest source is outside templates".to_owned())?;
        validate_relative_path(target)?;
        if previous.is_some_and(|value: &String| value >= path)
            || !exact.insert(path.clone())
            || !folded.insert(path.to_ascii_lowercase())
        {
            return Err("manifest source list is duplicate, aliased, or unordered".to_owned());
        }
        previous = Some(path);
    }
    Ok(())
}

fn validate_relative_path(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 512
        || !value.is_ascii()
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains('\\')
        || value
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".." | "~"))
    {
        return Err("template source path is not canonical".to_owned());
    }
    Ok(())
}

fn skip_whitespace(bytes: &[u8], index: &mut usize) {
    while bytes
        .get(*index)
        .is_some_and(|byte| matches!(byte, b' ' | b'\n' | b'\r' | b'\t'))
    {
        *index += 1;
    }
}

fn expect(bytes: &[u8], index: &mut usize, expected: u8) -> Result<(), String> {
    if bytes.get(*index) != Some(&expected) {
        return Err("manifest structure is malformed".to_owned());
    }
    *index += 1;
    Ok(())
}

#[cfg(unix)]
fn identity(metadata: &fs::Metadata) -> SourceIdentity {
    SourceIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        links: metadata.nlink(),
        length: metadata.len(),
        mode: metadata.permissions().mode() & 0o7777,
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

#[cfg(not(unix))]
fn identity(_metadata: &fs::Metadata) -> SourceIdentity {
    SourceIdentity {
        device: 0,
        inode: 0,
        links: 0,
        length: 0,
        mode: 0,
        modified_seconds: 0,
        modified_nanoseconds: 0,
        changed_seconds: 0,
        changed_nanoseconds: 0,
    }
}

#[cfg(target_os = "macos")]
const fn no_follow_nonblock_flags() -> i32 {
    0x0100 | 0x0004
}

#[cfg(any(target_os = "linux", target_os = "android"))]
const fn no_follow_nonblock_flags() -> i32 {
    0x20000 | 0x0800
}

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "linux", target_os = "android"))
))]
compile_error!(
    "repository-fit template source staging requires audited O_NOFOLLOW and O_NONBLOCK flags"
);
