use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use super::anchored::Session;
use crate::generated_authority::{
    GeneratedAuthorityParseRequest, GeneratedAuthorityRegistry, GeneratedSurface,
};

mod anchored;
#[cfg(test)]
mod hardening_tests;
#[cfg(test)]
mod tests;

pub(crate) const REGISTRY_PATH: &str = "migration/generated-surface-authority.json";
const MAX_REGISTRY_BYTES: u64 = 1024 * 1024;
const MAX_OUTPUT_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Classification {
    RetainedContext { replacement_targets: Vec<String> },
}

pub(crate) struct Catalog {
    rows: BTreeMap<String, GeneratedSurface>,
    registry_bytes: Arc<[u8]>,
}

pub(crate) trait Source {
    fn read(&mut self, relative: &str, maximum: u64) -> Result<Arc<[u8]>, String>;
}

impl Source for Session {
    fn read(&mut self, relative: &str, maximum: u64) -> Result<Arc<[u8]>, String> {
        Session::read(self, relative, maximum).map(Arc::from)
    }
}

impl Catalog {
    pub(crate) fn classify_many(
        root: &Path,
        paths: &[String],
    ) -> Result<BTreeMap<String, Classification>, String> {
        let mut ordered = paths.to_vec();
        ordered.sort();
        if ordered.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err("generated package classification paths are not unique".to_string());
        }
        if ordered.iter().any(|path| !generated_path(path)) {
            return Err("path is not a generated package path".to_string());
        }
        if ordered.is_empty() {
            return Ok(BTreeMap::new());
        }

        let mut session = Session::open(root)
            .map_err(|error| format!("generated disposition session unavailable: {error}"))?;
        let catalog = Self::load_in(&mut session)?;
        let mut classifications = BTreeMap::new();
        for path in ordered {
            let classification = catalog.classify_in(&mut session, &path)?;
            classifications.insert(path, classification);
        }
        session
            .finish()
            .map_err(|error| format!("generated disposition snapshot invalid: {error}"))?;
        Ok(classifications)
    }

    #[cfg(test)]
    pub(super) fn load(root: &Path) -> Result<Self, String> {
        let mut session = Session::open(root)?;
        let catalog = Self::load_in(&mut session)?;
        session.finish()?;
        Ok(catalog)
    }

    pub(crate) fn load_in(session: &mut Session) -> Result<Self, String> {
        Self::load_from(session)
    }

    pub(crate) fn load_from(source: &mut impl Source) -> Result<Self, String> {
        let bytes = source
            .read(REGISTRY_PATH, MAX_REGISTRY_BYTES)
            .map_err(|error| format!("generated disposition registry unavailable: {error}"))?;
        let parsed = crate::generated_authority::parse(GeneratedAuthorityParseRequest {
            bytes: bytes.as_ref(),
        })
        .map_err(|error| error.stable_text().to_string())?
        .registry;
        verify_projections(source, &parsed)?;
        Ok(Self {
            rows: parsed
                .surfaces
                .into_iter()
                .map(|(output, surface)| (output.as_str().to_owned(), surface))
                .collect(),
            registry_bytes: bytes,
        })
    }

    pub(crate) fn for_manifest_in(
        session: &mut Session,
        paths: &[String],
    ) -> Result<Option<Self>, String> {
        Self::for_manifest_from(session, paths)
    }

    pub(crate) fn for_manifest_from(
        source: &mut impl Source,
        paths: &[String],
    ) -> Result<Option<Self>, String> {
        if paths.iter().all(|path| !generated_path(path)) {
            return Ok(None);
        }
        Self::load_from(source).map(Some)
    }

    pub(crate) fn registry_bytes(&self) -> &[u8] {
        &self.registry_bytes
    }

    #[cfg(test)]
    pub(super) fn classify(&self, root: &Path, relative: &str) -> Result<Classification, String> {
        let mut session = Session::open(root)?;
        let classification = self.classify_in(&mut session, relative)?;
        session.finish()?;
        Ok(classification)
    }

    pub(crate) fn classify_in(
        &self,
        session: &mut Session,
        relative: &str,
    ) -> Result<Classification, String> {
        self.classify_from(session, relative)
    }

    pub(crate) fn classify_from(
        &self,
        source: &mut impl Source,
        relative: &str,
    ) -> Result<Classification, String> {
        if !generated_path(relative) {
            return Err("path is not a generated package path".to_string());
        }
        let row = self
            .rows
            .get(relative)
            .ok_or_else(|| "generated package path has no adopted disposition".to_string())?;
        match row {
            GeneratedSurface::CanonicalProjection { .. }
            | GeneratedSurface::SourceProjection { .. }
            | GeneratedSurface::ToolProjection { .. } => Err(
                "provenance-only projection cannot authorize package classification".to_string(),
            ),
            GeneratedSurface::RetainedContext {
                sha256,
                replacement_targets,
                ..
            } => {
                let bytes = source
                    .read(relative, MAX_OUTPUT_BYTES)
                    .map_err(|error| format!("retained generated context unavailable: {error}"))?;
                let actual = digest_hex(bytes.as_ref());
                if actual != sha256.lowercase_hex() {
                    return Err("retained generated context digest mismatch".to_string());
                }
                Ok(Classification::RetainedContext {
                    replacement_targets: replacement_targets.clone(),
                })
            }
        }
    }
}

fn verify_projections(
    source: &mut impl Source,
    parsed: &GeneratedAuthorityRegistry,
) -> Result<(), String> {
    source
        .read(
            parsed.registry_projection.generator.as_str(),
            MAX_OUTPUT_BYTES,
        )
        .map_err(|_| "generated registry projection generator is unavailable".to_string())?;
    for canonical_source in &parsed.registry_projection.canonical_sources {
        source
            .read(canonical_source.as_str(), MAX_OUTPUT_BYTES)
            .map_err(|_| {
                "generated registry projection canonical source is unavailable".to_string()
            })?;
    }
    for (output, row) in &parsed.surfaces {
        let (generator, canonical_sources, output_sha256) = match row {
            GeneratedSurface::SourceProjection {
                generator,
                canonical_sources,
                output_sha256,
                ..
            } => (Some(generator), canonical_sources, output_sha256),
            GeneratedSurface::ToolProjection {
                canonical_sources,
                output_sha256,
                ..
            } => (None, canonical_sources, output_sha256),
            GeneratedSurface::CanonicalProjection { .. } => {
                return Err(
                    "canonical projection cannot authorize current generated authority".to_string(),
                );
            }
            GeneratedSurface::RetainedContext { .. } => continue,
        };
        if let Some(generator) = generator {
            source
                .read(generator.as_str(), MAX_OUTPUT_BYTES)
                .map_err(|_| "source projection generator is unavailable".to_string())?;
        }
        for canonical_source in canonical_sources {
            source
                .read(canonical_source.as_str(), MAX_OUTPUT_BYTES)
                .map_err(|_| "projection canonical source is unavailable".to_string())?;
        }
        let bytes = source
            .read(output.as_str(), MAX_OUTPUT_BYTES)
            .map_err(|_| "projection output is unavailable".to_string())?;
        if digest_hex(bytes.as_ref()) != output_sha256.lowercase_hex() {
            return Err("projection output digest mismatch".to_string());
        }
    }
    Ok(())
}

fn digest_hex(bytes: &[u8]) -> String {
    crate::digest::bytes(bytes)
        .strip_prefix("sha256:")
        .expect("digest prefix")
        .to_string()
}

pub(crate) fn generated_path(relative: &str) -> bool {
    ["generated/", "docs/generated/", "examples/generated/"]
        .iter()
        .any(|prefix| relative.starts_with(prefix))
}
