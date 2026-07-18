use super::row::{self, PackageSurfaceRow};
use crate::package::inventory::generated_disposition::{Catalog, Classification, generated_path};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[cfg(all(test, unix))]
mod batch_tests;

pub(super) struct State {
    classifications: BTreeMap<String, Result<Classification, ()>>,
}

const FAILURE_SURFACE: &str = "generated-disposition-batch";
const INVALID_SURFACE_ID: &str = "generated-artifact:generated-disposition-batch-invalid";

impl State {
    pub(super) fn build(root: &Path, inventory: &BTreeSet<String>) -> Self {
        let generated = inventory
            .iter()
            .filter(|path| generated_path(path))
            .cloned()
            .collect::<Vec<_>>();
        if generated.is_empty() {
            return Self {
                classifications: BTreeMap::new(),
            };
        }
        let classifications = match Catalog::classify_many(root, &generated) {
            Ok(rows) => rows
                .into_iter()
                .map(|(path, classification)| (path, Ok(classification)))
                .collect(),
            Err(_) => generated.into_iter().map(|path| (path, Err(()))).collect(),
        };
        Self { classifications }
    }

    pub(super) fn row(&self, relative: &str) -> PackageSurfaceRow {
        match self.classifications.get(relative) {
            Some(Ok(Classification::RetainedContext {
                replacement_targets,
            })) => row::retained_context(relative, replacement_targets),
            Some(Err(_)) | None => {
                let mut invalid = row::invalid_generated_authority(relative);
                invalid.surface_id = INVALID_SURFACE_ID.to_string();
                invalid
            }
        }
    }

    pub(super) fn failures(&self) -> Vec<(String, String)> {
        let invalid_count = self
            .classifications
            .values()
            .filter(|result| result.is_err())
            .count();
        if invalid_count == 0 {
            return Vec::new();
        }
        vec![(
            FAILURE_SURFACE.to_string(),
            format!(
                "failure_class=generated_disposition_batch_invalid;invalid_generated_path_count={invalid_count}"
            ),
        )]
    }
}
