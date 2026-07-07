use super::surfaces::LoopValidationSurface;
use std::collections::BTreeMap;
use std::path::Path;

mod path_rules;

pub(crate) struct ChangedInputs {
    pub(crate) changed_files_digest: String,
    pub(crate) changed_file_count: usize,
    pub(crate) audit_context_digest: String,
    changed_files: Vec<String>,
    surface_digests: BTreeMap<&'static str, String>,
    affected_surfaces: BTreeMap<&'static str, bool>,
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
            format!("validator=ultragoal-rust;law=observability-live-loop;tier={tier};cache={cache_mode}")
                .as_bytes(),
        );
        let mut surface_digests = BTreeMap::new();
        let mut affected_surfaces = BTreeMap::new();
        for surface in super::surfaces::LOOP_VALIDATION_SURFACES {
            surface_digests.insert(
                surface.id,
                surface_changed_digest(root, *surface, candidate_digest, &changed_files),
            );
            affected_surfaces.insert(surface.id, surface_is_affected(*surface, &changed_files));
        }
        Self {
            changed_files_digest,
            changed_file_count,
            audit_context_digest,
            changed_files,
            surface_digests,
            affected_surfaces,
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
                    "changed_input_digest": self.surface_digest(*surface)
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
                    "reuse_condition": "verified_cache_hit_required_or_boundary_withheld"
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
        Self {
            changed_files_digest: changed_files_digest.to_string(),
            changed_file_count: 1,
            audit_context_digest: audit_context_digest.to_string(),
            changed_files: vec!["validator/src/cli/live_loop/mod.rs".to_string()],
            surface_digests,
            affected_surfaces,
        }
    }
}

fn bounded_changed_files(changed_files: &[String]) -> Vec<&str> {
    changed_files.iter().take(25).map(String::as_str).collect()
}

fn changed_files(root: &Path) -> Vec<String> {
    if !git_root_matches_requested_root(root) {
        return Vec::new();
    }
    let output = std::process::Command::new("git")
        .args(["status", "--short", "--untracked-files=all"])
        .current_dir(root)
        .output();
    output
        .ok()
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(changed_path)
                .collect()
        })
        .unwrap_or_default()
}

fn git_root_matches_requested_root(root: &Path) -> bool {
    let requested = root.canonicalize().ok();
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(root)
        .output()
        .ok();
    let git_root = output
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .and_then(|text| Path::new(&text).canonicalize().ok());
    requested
        .zip(git_root)
        .is_some_and(|(requested, git_root)| requested == git_root)
}

fn changed_path(line: &str) -> Option<String> {
    if line.trim().is_empty() {
        return None;
    }
    let path_part = line.get(3..).unwrap_or(line);
    let path = path_part
        .split_once(" -> ")
        .map(|(_, renamed)| renamed)
        .unwrap_or(path_part)
        .trim();
    (!path.is_empty()).then(|| path.to_string())
}

fn surface_changed_digest(
    root: &Path,
    surface: LoopValidationSurface,
    candidate_digest: &str,
    changed_files: &[String],
) -> String {
    match surface.id {
        "package_digest" => return candidate_digest.to_string(),
        "changed_files" => return crate::digest::bytes(changed_files.join("\n").as_bytes()),
        "audit_context" => {
            return crate::digest::bytes(
                format!("surface={};role=audit-context", surface.id).as_bytes(),
            );
        }
        _ => {}
    }
    let mut material = format!("surface={};", surface.id);
    let mut matched = false;
    for path in changed_files
        .iter()
        .filter(|path| path_affects_surface(path, surface.id))
    {
        matched = true;
        material.push_str(path);
        material.push('=');
        material.push_str(&file_digest(root, path));
        material.push(';');
    }
    if !matched {
        material.push_str("no-relevant-changed-inputs");
    }
    crate::digest::bytes(material.as_bytes())
}

fn surface_is_affected(surface: LoopValidationSurface, changed_files: &[String]) -> bool {
    !changed_files.is_empty()
        && changed_files
            .iter()
            .any(|path| path_affects_surface(path, surface.id))
}

fn file_digest(root: &Path, path: &str) -> String {
    let full_path = root.join(path);
    match std::fs::read(&full_path) {
        Ok(bytes) => crate::digest::bytes(&bytes),
        Err(_) => crate::digest::bytes(format!("missing:{path}").as_bytes()),
    }
}

fn path_affects_surface(path: &str, surface_id: &str) -> bool {
    path_rules::path_affects_surface(path, surface_id)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
