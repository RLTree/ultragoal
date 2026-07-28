use super::*;

#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd};

#[cfg(unix)]
unsafe extern "C" {
    fn openat(directory: i32, path: *const i8, flags: i32, ...) -> i32;
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) const DIRECTORY_FLAG: i32 = 0x10000;
#[cfg(target_os = "macos")]
pub(crate) const DIRECTORY_FLAG: i32 = 0x100000;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) const CREATE_EXCLUSIVE_FLAGS: i32 = 0x40 | 0x80;
#[cfg(target_os = "macos")]
pub(crate) const CREATE_EXCLUSIVE_FLAGS: i32 = 0x0200 | 0x0800;

#[cfg(unix)]
pub(crate) fn open_staged_leaf(
    parent: &File,
    name: &CString,
    flags: i32,
    mode: u32,
) -> Result<File, std::io::Error> {
    // SAFETY: parent is a live directory descriptor, name is NUL-terminated,
    // and ownership of a successful descriptor is transferred to File.
    let descriptor = unsafe { openat(parent.as_raw_fd(), name.as_ptr(), flags, mode as i32) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: openat returned one newly owned descriptor.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub(crate) const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
pub(crate) const MAX_SOURCE_BYTES: u64 = 8 * 1024 * 1024;
pub(crate) const MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const MAX_ROWS: usize = 512;
pub(crate) const GENERATED_CATALOG: &str = "repository_fit_template_catalog.rs";
pub(crate) const STAGING_DIRECTORY: &str = "repository_fit_template_sources";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagedTemplate {
    pub source_path: String,
    pub target_path: String,
    pub bytes: Vec<u8>,
    pub unix_mode: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) links: u64,
    pub(crate) length: u64,
    pub(crate) mode: u32,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanoseconds: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanoseconds: i64,
}

#[derive(Clone, Debug)]
pub(crate) struct ObservedSource {
    pub(crate) source_path: String,
    pub(crate) target_path: String,
    pub(crate) absolute: PathBuf,
    pub(crate) identity: SourceIdentity,
}

#[derive(Clone, Debug)]
pub(crate) struct ObservedTree {
    pub(crate) directories: BTreeMap<String, SourceIdentity>,
    pub(crate) sources: BTreeMap<String, ObservedSource>,
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

pub(crate) fn generate_after_cleanup<F, G, H>(
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
    generated.push_str("pub(crate) static CANONICAL_TEMPLATES: &[TemplateCatalogRow] = &[\n");
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

#[cfg(test)]
pub(crate) fn stage_manifest_sources(
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

#[cfg(all(test, unix))]
mod publication_tests {
    use std::os::unix::fs::symlink;

    #[test]
    fn staged_publication_writes_a_new_regular_file() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
            "staged-publication-positive",
        );
        std::fs::create_dir_all(&root).expect("root");
        let staged = root.join("staged.bin");
        super::write_staged(&staged, b"trusted bytes").expect("publish");
        assert_eq!(std::fs::read(&staged).expect("read"), b"trusted bytes");
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn staged_publication_rejects_leaf_and_ancestor_symlinks() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
            "staged-publication-symlinks",
        );
        let outside = root.join("outside");
        std::fs::create_dir_all(&outside).expect("outside");
        let victim = outside.join("victim");
        let staged = root.join("staged.bin");
        std::fs::write(&victim, b"unchanged").expect("victim");
        symlink(&victim, &staged).expect("leaf symlink");
        assert!(super::write_staged(&staged, b"attacker bytes").is_err());
        assert_eq!(std::fs::read(&victim).expect("victim read"), b"unchanged");

        let linked_parent = root.join("linked-parent");
        symlink(&outside, &linked_parent).expect("ancestor symlink");
        assert!(
            super::write_staged(&linked_parent.join("created.bin"), b"attacker bytes").is_err()
        );
        assert!(!outside.join("created.bin").exists());
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn staged_publication_path_verification_rejects_a_replaced_parent() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
            "staged-publication-replaced-parent",
        );
        let parent = root.join("parent");
        std::fs::create_dir_all(&parent).expect("parent");
        let path = parent.join("staged.bin");
        std::fs::write(&path, b"trusted").expect("leaf");
        let (opened_parent, _) = super::open_staged_parent(&path).expect("open parent");
        let expected_leaf = std::fs::metadata(&path).expect("leaf metadata");
        std::fs::rename(&parent, root.join("moved-parent")).expect("replace parent");
        std::fs::create_dir(&parent).expect("replacement");
        assert!(
            super::verify_staged_publication_path(&path, &opened_parent, &expected_leaf).is_err()
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
