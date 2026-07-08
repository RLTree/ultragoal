use super::surfaces::{LoopValidationSurface, input_spec_for};
use std::collections::BTreeMap;
use std::path::Path;

mod git_status;
mod surface_inputs;

use git_status::changed_files;
use surface_inputs::{
    invalidation_reason, path_affects_surface, surface_changed_digest, surface_is_affected,
};

#[cfg(test)]
pub(super) use git_status::{changed_path, git_root_matches_requested_root};
#[cfg(test)]
pub(super) use surface_inputs::file_digest;

pub(crate) struct ChangedInputs {
    pub(crate) changed_files_digest: String,
    pub(crate) changed_file_count: usize,
    pub(crate) audit_context_digest: String,
    changed_files: Vec<String>,
    surface_digests: BTreeMap<&'static str, String>,
    affected_surfaces: BTreeMap<&'static str, bool>,
    invalidation_reasons: BTreeMap<&'static str, &'static str>,
}

impl ChangedInputs {
    pub(crate) fn collect(
        root: &Path,
        candidate_digest: &str,
        tier: &str,
        cache_mode: &str,
    ) -> Self {
        let changed_files = changed_files(root);
        let changed_file_count = changed_files.len();
        let changed_files_digest = crate::digest::bytes(changed_files.join("\n").as_bytes());
        let audit_context_digest = crate::digest::bytes(
            format!(
                "validator={};law={};schema={};fixture={};tier={tier};cache={cache_mode}",
                super::graph::validator_version(),
                super::graph::law_version(),
                super::graph::schema_version(),
                super::graph::fixture_version()
            )
            .as_bytes(),
        );
        let mut surface_digests = BTreeMap::new();
        let mut affected_surfaces = BTreeMap::new();
        let mut invalidation_reasons = BTreeMap::new();
        for surface in super::surfaces::LOOP_VALIDATION_SURFACES {
            surface_digests.insert(
                surface.id,
                surface_changed_digest(root, *surface, candidate_digest, &changed_files),
            );
            affected_surfaces.insert(surface.id, surface_is_affected(*surface, &changed_files));
            invalidation_reasons.insert(surface.id, invalidation_reason(*surface, &changed_files));
        }
        Self {
            changed_files_digest,
            changed_file_count,
            audit_context_digest,
            changed_files,
            surface_digests,
            affected_surfaces,
            invalidation_reasons,
        }
    }

    pub(crate) fn surface_digest(&self, surface: LoopValidationSurface) -> &str {
        self.surface_digests
            .get(surface.id)
            .map(String::as_str)
            .unwrap_or(&self.changed_files_digest)
    }

    pub(crate) fn affects_surface(&self, surface: LoopValidationSurface) -> bool {
        self.affected_surfaces
            .get(surface.id)
            .copied()
            .unwrap_or(false)
    }

    pub(crate) fn surface_changed_paths(&self, surface_id: &str) -> Vec<String> {
        self.changed_files
            .iter()
            .filter(|path| path_affects_surface(path, surface_id))
            .cloned()
            .collect()
    }

    pub(crate) fn invalidation_reason(&self, surface: LoopValidationSurface) -> &'static str {
        self.invalidation_reasons
            .get(surface.id)
            .copied()
            .unwrap_or("missing_surface_input_spec")
    }

    pub(crate) fn affected_high_frequency_surfaces(&self) -> Vec<LoopValidationSurface> {
        super::surfaces::LOOP_VALIDATION_SURFACES
            .iter()
            .copied()
            .filter(|surface| surface.high_frequency && self.affects_surface(*surface))
            .collect()
    }

    pub(crate) fn summary(&self) -> serde_json::Value {
        let affected_nodes: Vec<serde_json::Value> = super::surfaces::LOOP_VALIDATION_SURFACES
            .iter()
            .filter(|surface| self.affects_surface(**surface))
            .map(|surface| {
                serde_json::json!({
                    "node_id": surface.id,
                    "surface": surface.surface,
                    "changed_input_digest": self.surface_digest(*surface),
                    "invalidation_reason": self.invalidation_reason(*surface)
                })
            })
            .collect();
        let unaffected_nodes: Vec<serde_json::Value> = super::surfaces::LOOP_VALIDATION_SURFACES
            .iter()
            .filter(|surface| !self.affects_surface(**surface))
            .map(|surface| {
                serde_json::json!({
                    "node_id": surface.id,
                    "surface": surface.surface,
                    "reuse_condition": input_spec_for(surface.id)
                        .map(|spec| spec.unaffected_reuse_condition())
                        .unwrap_or("missing_surface_input_spec_blocks_cache_reuse")
                })
            })
            .collect();
        serde_json::json!({
            "changed_file_count": self.changed_file_count,
            "changed_files_digest": self.changed_files_digest,
            "changed_files": bounded_changed_files(&self.changed_files),
            "changed_files_truncated": self.changed_files.len() > 25,
            "affected_node_count": affected_nodes.len(),
            "affected_nodes": affected_nodes,
            "unaffected_node_count": unaffected_nodes.len(),
            "unaffected_nodes": unaffected_nodes
        })
    }

    #[cfg(test)]
    pub(crate) fn for_tests(changed_files_digest: &str, audit_context_digest: &str) -> Self {
        let surface_digests = super::surfaces::LOOP_VALIDATION_SURFACES
            .iter()
            .map(|surface| (surface.id, changed_files_digest.to_string()))
            .collect();
        let affected_surfaces = super::surfaces::LOOP_VALIDATION_SURFACES
            .iter()
            .map(|surface| (surface.id, true))
            .collect();
        let invalidation_reasons = super::surfaces::LOOP_VALIDATION_SURFACES
            .iter()
            .map(|surface| (surface.id, "covered_input_mutation"))
            .collect();
        Self {
            changed_files_digest: changed_files_digest.to_string(),
            changed_file_count: 1,
            audit_context_digest: audit_context_digest.to_string(),
            changed_files: vec!["validator/src/cli/live_loop/mod.rs".to_string()],
            surface_digests,
            affected_surfaces,
            invalidation_reasons,
        }
    }
}

fn bounded_changed_files(changed_files: &[String]) -> Vec<&str> {
    changed_files.iter().take(25).map(String::as_str).collect()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
