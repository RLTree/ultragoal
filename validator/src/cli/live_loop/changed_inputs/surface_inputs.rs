use super::super::surfaces::{CacheBoundary, LoopValidationSurface, input_spec_for};
use std::path::Path;

pub(super) fn surface_changed_digest(
    root: &Path,
    surface: LoopValidationSurface,
    candidate_digest: &str,
    changed_files: &[String],
) -> String {
    match surface.id {
        "package_digest" => return candidate_digest.to_string(),
        "changed_files" => return crate::digest::bytes(changed_files.join("\n").as_bytes()),
        "audit_context" => return audit_context_digest(surface),
        _ => {}
    }
    let Some(spec) = input_spec_for(surface.id) else {
        return crate::digest::bytes(
            format!("missing-surface-input-spec:{}", surface.id).as_bytes(),
        );
    };
    let mut material = format!(
        "surface={};{};",
        surface.id,
        spec.cache_material("snapshot", "verified-local")
    );
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
        material.push_str(spec.no_changed_input_reason());
    }
    crate::digest::bytes(material.as_bytes())
}

fn audit_context_digest(surface: LoopValidationSurface) -> String {
    crate::digest::bytes(
        format!(
            "surface={};role=audit-context;{}",
            surface.id,
            input_spec_for(surface.id)
                .map(|spec| spec.cache_material("snapshot", "verified-local"))
                .unwrap_or_else(|| "missing_surface_input_spec".to_string())
        )
        .as_bytes(),
    )
}

pub(super) fn surface_is_affected(
    surface: LoopValidationSurface,
    changed_files: &[String],
) -> bool {
    input_spec_for(surface.id).is_some_and(|spec| match spec.cache_boundary {
        CacheBoundary::CandidatePackage => changed_files.iter().any(|path| spec.affects_path(path)),
        CacheBoundary::AuditContext | CacheBoundary::ChangedInputs => {
            !changed_files.is_empty() && changed_files.iter().any(|path| spec.affects_path(path))
        }
    })
}

pub(super) fn invalidation_reason(
    surface: LoopValidationSurface,
    changed_files: &[String],
) -> &'static str {
    let Some(spec) = input_spec_for(surface.id) else {
        return "missing_surface_input_spec";
    };
    if changed_files.iter().any(|path| spec.affects_path(path)) {
        return "covered_input_mutation";
    }
    spec.no_changed_input_reason()
}

pub(crate) fn file_digest(root: &Path, path: &str) -> String {
    let full_path = root.join(path);
    match std::fs::read(&full_path) {
        Ok(bytes) => crate::digest::bytes(&bytes),
        Err(_) => crate::digest::bytes(format!("missing:{path}").as_bytes()),
    }
}

pub(super) fn path_affects_surface(path: &str, surface_id: &str) -> bool {
    input_spec_for(surface_id).is_some_and(|spec| spec.affects_path(path))
}
