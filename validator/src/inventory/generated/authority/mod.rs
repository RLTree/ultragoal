use crate::context::ReadSession;
use crate::generated_authority::{GeneratedAuthorityParseRequest, GeneratedSurface};
use crate::inventory::fs::{PhysicalEntryDescriptor, physical_entry, read_bounded};
use crate::inventory::types::{ActiveStatus, AuthorityState, InventoryEntry, InventoryError};
use std::collections::BTreeMap;
use std::path::Path;

mod adopted_schema_contract;
mod verification;

pub(super) use crate::generated_authority::GeneratedSurface as SurfaceSpec;

pub(super) const REGISTRY_PATH: &str = "migration/generated-surface-authority.json";
const MAX_REGISTRY_BYTES: u64 = 1024 * 1024;

fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}

pub(super) fn load(
    reads: &ReadSession,
    root: &Path,
    contract_id: &str,
) -> Result<(BTreeMap<String, SurfaceSpec>, InventoryEntry), InventoryError> {
    let path = root.join(REGISTRY_PATH);
    let bytes = read_bounded(reads, &path, MAX_REGISTRY_BYTES)?;
    let parsed =
        crate::generated_authority::parse(GeneratedAuthorityParseRequest { bytes: &bytes })
            .map_err(|error| invalid(error.stable_text()))?
            .registry;
    if contract_id != "harness-ultragoal-successor-contract-v2" {
        return Err(invalid("generated authority contract ID is unsupported"));
    }
    verification::verify(reads, root, &parsed)?;
    let entry = physical_entry(
        reads,
        root,
        &path,
        PhysicalEntryDescriptor {
            stable_id: "GENERATED-SURFACE-AUTHORITY".to_owned(),
            kind: "generated-surface-authority-registry",
            owner: "OWN-ULTRA-ROOT",
            authority_state: AuthorityState::Canonical,
            active_status: ActiveStatus::Active,
            generator: None,
            provenance: Vec::new(),
            references: Vec::new(),
        },
    )?;
    Ok((
        parsed
            .surfaces
            .into_iter()
            .map(|(output, surface): (_, GeneratedSurface)| (output.as_str().to_owned(), surface))
            .collect(),
        entry,
    ))
}
