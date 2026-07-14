use super::*;

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
