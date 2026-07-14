use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

use crate::context::LiveContext;

use super::projection::{TemplateAuthorityProjection, TemplateRowProjection};
use super::{AdapterErrorId, FitAdapterError, adapter_error};
use crate::repository_fit::{CanonicalPath, DesiredFile, DesiredState, digest};

const MAX_MANIFEST_BYTES: usize = 8 * 1024 * 1024;
const MAX_CATALOG_BYTES: usize = 64 * 1024 * 1024;
const MAX_TEMPLATE_ROWS: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TemplateSourceKind {
    Regular,
    #[cfg(test)]
    Symlink,
    #[cfg(test)]
    Hardlink,
    #[cfg(test)]
    Directory,
    #[cfg(test)]
    Fifo,
    #[cfg(test)]
    Socket,
    #[cfg(test)]
    CrossDevice,
}

#[derive(Clone, Copy)]
pub(super) struct TemplateCatalogRow {
    pub(super) source_path: &'static str,
    pub(super) target_path: &'static str,
    pub(super) bytes: &'static [u8],
    pub(super) unix_mode: u32,
    pub(super) source_kind: TemplateSourceKind,
}

impl TemplateCatalogRow {
    const fn regular(
        source_path: &'static str,
        target_path: &'static str,
        bytes: &'static [u8],
        unix_mode: u32,
    ) -> Self {
        Self {
            source_path,
            target_path,
            bytes,
            unix_mode,
            source_kind: TemplateSourceKind::Regular,
        }
    }
}

// The build script classifies and stages every real manifest-authorized source
// before rustc can embed it. This generated file references only private
// OUT_DIR copies and carries the exact inspected source path and Unix mode.
include!(concat!(
    env!("OUT_DIR"),
    "/repository_fit_template_catalog.rs"
));

pub(super) struct DesiredBundle {
    pub(super) desired: DesiredState,
    pub(super) authority: TemplateAuthorityProjection,
    pub(super) unix_modes: BTreeMap<String, u32>,
}

pub(super) fn compile(context: &LiveContext) -> Result<DesiredBundle, FitAdapterError> {
    compile_rows(context, CANONICAL_TEMPLATES, MANIFEST_BYTES)
}

pub(super) fn compile_rows(
    context: &LiveContext,
    rows: &[TemplateCatalogRow],
    manifest_bytes: &[u8],
) -> Result<DesiredBundle, FitAdapterError> {
    if rows.is_empty()
        || rows.len() > MAX_TEMPLATE_ROWS
        || manifest_bytes.is_empty()
        || manifest_bytes.len() > MAX_MANIFEST_BYTES
    {
        return Err(adapter_error(AdapterErrorId::InvalidTemplateCatalog));
    }
    let manifest_paths = manifest_paths(manifest_bytes)?;
    validate_manifest_order(&manifest_paths)?;
    let manifest_folded = folded_set(&manifest_paths)?;
    let manifest_exact = manifest_paths.iter().cloned().collect::<BTreeSet<_>>();
    let manifest_sha256 = digest(manifest_bytes);
    let candidate_id = digest(
        &serde_json::to_vec(context.candidate())
            .map_err(|_| adapter_error(AdapterErrorId::InvalidTemplateCatalog))?,
    );

    let mut desired_files = Vec::with_capacity(rows.len());
    let mut projections = Vec::with_capacity(rows.len());
    let mut catalog_sources_exact = BTreeSet::new();
    let mut catalog_sources_folded = BTreeSet::new();
    let mut catalog_targets = BTreeSet::new();
    let mut unix_modes = BTreeMap::new();
    let mut total_bytes = 0usize;
    for row in rows {
        if row.source_kind != TemplateSourceKind::Regular
            || !matches!(row.unix_mode, 0o644 | 0o755)
            || row.source_path != format!("templates/{}", row.target_path)
        {
            return Err(adapter_error(AdapterErrorId::InvalidTemplateCatalog));
        }
        let target = CanonicalPath::parse(row.target_path)
            .map_err(|_| adapter_error(AdapterErrorId::InvalidTemplateCatalog))?;
        let source = CanonicalPath::parse(row.source_path)
            .map_err(|_| adapter_error(AdapterErrorId::InvalidTemplateCatalog))?;
        if !source.as_str().starts_with("templates/")
            || !catalog_sources_exact.insert(source.as_str().to_owned())
            || !catalog_sources_folded.insert(source.folded())
            || !catalog_targets.insert(target.folded())
            || unix_modes
                .insert(target.as_str().to_owned(), row.unix_mode)
                .is_some()
        {
            return Err(adapter_error(AdapterErrorId::InvalidTemplateCatalog));
        }
        total_bytes = total_bytes
            .checked_add(row.bytes.len())
            .ok_or_else(|| adapter_error(AdapterErrorId::InvalidTemplateCatalog))?;
        if total_bytes > MAX_CATALOG_BYTES {
            return Err(adapter_error(AdapterErrorId::InvalidTemplateCatalog));
        }
        let row_sha256 = row_digest(row)?;
        let sha256 = digest(row.bytes);
        projections.push(TemplateRowProjection {
            source_path: row.source_path.to_owned(),
            target_path: row.target_path.to_owned(),
            sha256,
            byte_length: row.bytes.len(),
            unix_mode: row.unix_mode,
            row_sha256: row_sha256.clone(),
        });
        desired_files.push(
            DesiredFile::managed(
                target,
                row.bytes.to_vec(),
                context.context_id().to_owned(),
                candidate_id.clone(),
                manifest_sha256.clone(),
                row_sha256,
                Vec::<String>::new(),
            )
            .map_err(super::kernel_error)?,
        );
    }
    if catalog_sources_exact != manifest_exact || catalog_sources_folded != manifest_folded {
        return Err(adapter_error(AdapterErrorId::InvalidTemplateCatalog));
    }
    projections.sort_by(|left, right| left.target_path.cmp(&right.target_path));
    let catalog_sha256 = digest(
        &serde_json::to_vec(&projections)
            .map_err(|_| adapter_error(AdapterErrorId::InvalidTemplateCatalog))?,
    );
    let authority_sha256 = digest(
        &serde_json::to_vec(&(
            "repository-fit-template-authority-v1",
            &manifest_sha256,
            &catalog_sha256,
        ))
        .map_err(|_| adapter_error(AdapterErrorId::InvalidTemplateCatalog))?,
    );
    let authority = TemplateAuthorityProjection {
        manifest_sha256,
        catalog_sha256,
        authority_sha256,
        template_count: projections.len(),
        total_bytes,
        rows: projections,
    };
    let desired = DesiredState::new(context.context_id().to_owned(), candidate_id, desired_files)
        .map_err(super::kernel_error)?;
    Ok(DesiredBundle {
        desired,
        authority,
        unix_modes,
    })
}

fn manifest_paths(bytes: &[u8]) -> Result<Vec<String>, FitAdapterError> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| adapter_error(AdapterErrorId::InvalidTemplateCatalog))?;
    let object = value
        .as_object()
        .ok_or_else(|| adapter_error(AdapterErrorId::InvalidTemplateCatalog))?;
    let rows = object
        .get("authorable_templates")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| adapter_error(AdapterErrorId::InvalidTemplateCatalog))?;
    if rows.is_empty() || rows.len() > MAX_TEMPLATE_ROWS {
        return Err(adapter_error(AdapterErrorId::InvalidTemplateCatalog));
    }
    rows.iter()
        .map(|row| {
            row.as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| adapter_error(AdapterErrorId::InvalidTemplateCatalog))
        })
        .collect()
}

fn validate_manifest_order(paths: &[String]) -> Result<(), FitAdapterError> {
    if paths.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(adapter_error(AdapterErrorId::InvalidTemplateCatalog));
    }
    Ok(())
}

fn folded_set(paths: &[String]) -> Result<BTreeSet<String>, FitAdapterError> {
    let mut set = BTreeSet::new();
    for path in paths {
        let parsed = CanonicalPath::parse(path)
            .map_err(|_| adapter_error(AdapterErrorId::InvalidTemplateCatalog))?;
        if !parsed.as_str().starts_with("templates/") || !set.insert(parsed.folded()) {
            return Err(adapter_error(AdapterErrorId::InvalidTemplateCatalog));
        }
    }
    Ok(set)
}

fn row_digest(row: &TemplateCatalogRow) -> Result<String, FitAdapterError> {
    #[derive(Serialize)]
    struct Row<'a> {
        source_path: &'a str,
        target_path: &'a str,
        sha256: String,
        byte_length: usize,
        unix_mode: u32,
    }
    serde_json::to_vec(&Row {
        source_path: row.source_path,
        target_path: row.target_path,
        sha256: digest(row.bytes),
        byte_length: row.bytes.len(),
        unix_mode: row.unix_mode,
    })
    .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)))
    .map_err(|_| adapter_error(AdapterErrorId::InvalidTemplateCatalog))
}
