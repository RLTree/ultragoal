use super::authority;
use super::authority::SurfaceSpec;
use super::metadata;
use super::retained;
use crate::context::ReadSession;
use crate::inventory::fs::{
    PhysicalEntryDescriptor, check_symlink, physical_entry, physical_regular_entry, regular_files,
    relative,
};
use crate::inventory::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn error(findings: &mut Vec<InventoryFinding>, code: &str, relative: &str, message: &str) {
    findings.push(InventoryFinding::error(
        code,
        Some(&format!("GENERATED:{relative}")),
        Some(relative),
        message.to_owned(),
    ));
}

fn retained_absence(root: &Path, output: &str) -> (&'static str, &'static str) {
    match fs::symlink_metadata(root.join(output)) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (
            "retained_context_output_missing",
            "retained generated context output is missing",
        ),
        Ok(metadata) if metadata.file_type().is_symlink() => (
            "retained_context_output_symlink",
            "retained generated context output must not be a symlink",
        ),
        Ok(metadata) if !metadata.is_file() => (
            "retained_context_output_not_regular",
            "retained generated context output is not a regular file",
        ),
        _ => (
            "retained_context_output_unavailable",
            "retained generated context output was not safely discovered",
        ),
    }
}

pub(crate) fn discover(
    reads: &ReadSession,
    root: &Path,
    contract_id: &str,
    entries: &mut Vec<InventoryEntry>,
    findings: &mut Vec<InventoryFinding>,
) -> Result<(), InventoryError> {
    let (authority, authority_entry) = authority::load(reads, root, contract_id)?;
    entries.push(authority_entry);
    let mut seen = BTreeSet::new();
    for directory in ["generated", "docs/generated", "examples/generated"] {
        for path in regular_files(reads, root, directory)? {
            let confined = check_symlink(root, &path, findings)?;
            let rel = relative(root, &path)?;
            if !confined {
                continue;
            }
            seen.insert(rel.clone());
            if let Some(SurfaceSpec::RetainedContext {
                sha256,
                replacement_targets,
                ..
            }) = authority.get(&rel)
            {
                let sha256 = sha256.lowercase_hex();
                let inspected = retained::inspect(reads, &path, &sha256, replacement_targets);
                for (code, message) in &inspected.problems {
                    error(findings, code, &rel, message);
                }
                let (authority_state, active_status, references) = if inspected.problems.is_empty()
                {
                    (
                        AuthorityState::Context,
                        ActiveStatus::ContextOnly,
                        inspected.replacement_targets,
                    )
                } else {
                    (AuthorityState::Legacy, ActiveStatus::Active, Vec::new())
                };
                entries.push(physical_regular_entry(
                    reads,
                    root,
                    &path,
                    PhysicalEntryDescriptor {
                        stable_id: format!("GENERATED:{rel}"),
                        kind: "generated-surface",
                        owner: "OWN-ULTRA-ROOT",
                        authority_state,
                        active_status,
                        generator: None,
                        provenance: vec![authority::REGISTRY_PATH.to_owned()],
                        references,
                    },
                )?);
                continue;
            }
            if let Some(spec) = authority.get(&rel) {
                let projection_sources = match spec {
                    SurfaceSpec::SourceProjection {
                        canonical_sources, ..
                    }
                    | SurfaceSpec::ToolProjection {
                        canonical_sources, ..
                    } => Some(
                        canonical_sources
                            .iter()
                            .map(|path| path.as_str().to_owned())
                            .collect(),
                    ),
                    _ => None,
                };
                if let Some(provenance) = projection_sources {
                    entries.push(physical_regular_entry(
                        reads,
                        root,
                        &path,
                        PhysicalEntryDescriptor {
                            stable_id: format!("GENERATED:{rel}"),
                            kind: "provenance-only-projection",
                            owner: "OWN-ULTRA-ROOT",
                            authority_state: AuthorityState::Context,
                            active_status: ActiveStatus::ContextOnly,
                            generator: None,
                            provenance,
                            references: vec![authority::REGISTRY_PATH.to_owned()],
                        },
                    )?);
                    continue;
                }
            }
            let metadata = metadata::inspect(reads, root, &path, authority.get(&rel));
            for (code, message) in &metadata.problems {
                error(findings, code, &rel, message);
            }
            if metadata.generator.is_none() {
                findings.push(InventoryFinding::warning(
                    "generated_surface_missing_provenance",
                    Some(&format!("GENERATED:{rel}")),
                    Some(&rel),
                    "generated surface has no generator metadata".to_owned(),
                ));
            }
            entries.push(physical_entry(
                reads,
                root,
                &path,
                PhysicalEntryDescriptor {
                    stable_id: format!("GENERATED:{rel}"),
                    kind: "generated-surface",
                    owner: "OWN-PRODUCT-ARCHITECTURE",
                    authority_state: AuthorityState::Projection,
                    active_status: ActiveStatus::ContextOnly,
                    generator: metadata.generator.clone(),
                    provenance: metadata.inputs,
                    references: metadata.generator.into_iter().collect(),
                },
            )?);
        }
    }
    for (output, spec) in authority
        .iter()
        .filter(|(output, _)| !seen.contains(*output))
    {
        match spec {
            SurfaceSpec::CanonicalProjection { .. } => error(
                findings,
                "registered_generated_surface_missing",
                output,
                "externally authorized generated output is missing",
            ),
            SurfaceSpec::RetainedContext { .. } => {
                let (code, message) = retained_absence(root, output);
                error(findings, code, output, message);
            }
            SurfaceSpec::SourceProjection { .. } | SurfaceSpec::ToolProjection { .. } => {}
        }
    }
    Ok(())
}
