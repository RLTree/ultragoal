use super::contract_codec::{CoverageManifest, Digest};
use std::fmt;
use std::path::{Path, PathBuf};

pub(crate) enum CoverageSourceDigestRequest<'a> {
    SourceTree {
        root: &'a Path,
        manifest: &'a CoverageManifest,
    },
    ChangedFiles {
        root: &'a Path,
        manifest: &'a CoverageManifest,
    },
}

pub(crate) struct CoverageSourceDigestResponse {
    pub(crate) digest: Digest,
}

#[derive(Debug)]
pub(crate) struct CoverageSourceDigestError {
    pub(crate) code: &'static str,
    pub(crate) detail: String,
}

impl fmt::Display for CoverageSourceDigestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.code, self.detail)
    }
}

pub(crate) fn execute(
    request: CoverageSourceDigestRequest<'_>,
) -> Result<CoverageSourceDigestResponse, CoverageSourceDigestError> {
    let (root, mut files) = match request {
        CoverageSourceDigestRequest::SourceTree { root, manifest } => {
            let root = canonical_root(root)?;
            let files = source_files(&root, manifest)?;
            (root, files)
        }
        CoverageSourceDigestRequest::ChangedFiles { root, manifest } => {
            (canonical_root(root)?, manifest.changed_files())
        }
    };
    files.sort();
    files.dedup();
    let mut payload = Vec::new();
    for relative in files {
        payload.extend_from_slice(relative.as_bytes());
        payload.push(0);
        let path = root.join(&relative);
        let bytes = crate::digest::read_file_bytes(&path).map_err(|detail| {
            digest_error(
                "coverage_source_digest_read_failed",
                format!("{relative}:{detail}"),
            )
        })?;
        payload.extend_from_slice(&bytes);
        payload.push(0);
    }
    Ok(CoverageSourceDigestResponse {
        digest: Digest::new(crate::digest::bytes(&payload)),
    })
}

fn source_files(
    root: &Path,
    manifest: &CoverageManifest,
) -> Result<Vec<String>, CoverageSourceDigestError> {
    let mut files = Vec::new();
    let ignore_patterns = manifest.ignore_patterns();
    for target in manifest.target_paths() {
        let base = crate::package::inventory::resolve(root, &target)
            .map_err(|detail| digest_error("coverage_target_path_invalid", detail))?;
        if base.is_file() {
            files.push(target);
            continue;
        }
        for entry in walkdir::WalkDir::new(&base) {
            let entry = entry.map_err(|error| {
                digest_error(
                    "coverage_source_walk_failed",
                    format!("{}:{error}", base.display()),
                )
            })?;
            if entry.file_type().is_file() {
                let relative = relative_path(root, entry.path())?;
                if !ignored(&relative, &ignore_patterns) {
                    files.push(relative);
                }
            }
        }
    }
    Ok(files)
}

fn ignored(relative: &str, patterns: &[String]) -> bool {
    if crate::package::inventory::builder_contract_resource_path(relative) {
        return true;
    }
    if matches!(
        relative,
        "templates/.harness/coverage-manifest.json"
            | "templates/.harness/coverage-command"
            | ".harness/coverage-manifest.json"
            | ".harness/coverage-command"
    ) {
        return true;
    }
    patterns.iter().any(|pattern| {
        relative == pattern
            || pattern.strip_suffix("/**").is_some_and(|prefix| {
                relative == prefix || relative.starts_with(&format!("{prefix}/"))
            })
    })
}

fn canonical_root(root: &Path) -> Result<PathBuf, CoverageSourceDigestError> {
    root.canonicalize()
        .map_err(|error| digest_error("coverage_root_canonicalize_failed", error.to_string()))
}

fn relative_path(root: &Path, path: &Path) -> Result<String, CoverageSourceDigestError> {
    path.strip_prefix(root)
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .map_err(|error| digest_error("coverage_source_path_escape", error.to_string()))
}

fn digest_error(code: &'static str, detail: String) -> CoverageSourceDigestError {
    CoverageSourceDigestError { code, detail }
}
