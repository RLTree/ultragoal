use super::{BehavioralRole, CanonicalTarget, bindings};
use crate::context::ReadSession;
use crate::inventory::digest::file_identity_regular;
use crate::inventory::fs::{PhysicalEntryDescriptor, physical_regular_entry};
use crate::inventory::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use std::path::Path;

pub(crate) fn discover(
    reads: &ReadSession,
    root: &Path,
) -> Result<(Vec<InventoryEntry>, Vec<InventoryFinding>), InventoryError> {
    let mut entries = Vec::with_capacity(10);
    let mut findings = Vec::new();
    for binding in bindings() {
        let path = root.join(binding.relative_path);
        if !path.exists() {
            findings.push(InventoryFinding::error(
                "missing_behavioral_role_surface",
                Some(&stable_id(binding.role, binding.relative_path)),
                Some(binding.relative_path),
                "declared behavioral role surface is missing".to_owned(),
            ));
            continue;
        }
        let (authority_state, active_status) = match binding.role {
            BehavioralRole::TestContext => (AuthorityState::Context, ActiveStatus::ContextOnly),
            _ => (AuthorityState::Canonical, ActiveStatus::Active),
        };
        let mut provenance = Vec::new();
        let references = match binding.canonical_target {
            CanonicalTarget::StableId(stable_id) => vec![stable_id.to_owned()],
            CanonicalTarget::RelativePath(relative_path) => {
                let target = root.join(relative_path);
                if !target.exists() {
                    findings.push(InventoryFinding::error(
                        "missing_behavioral_role_target",
                        Some(&stable_id(binding.role, binding.relative_path)),
                        Some(relative_path),
                        format!(
                            "{} requires exact target {}",
                            binding.relative_path, relative_path
                        ),
                    ));
                    Vec::new()
                } else {
                    let (digest, _) = file_identity_regular(reads, &target)?;
                    provenance.push(format!("canonical-target:{relative_path}#sha256:{digest}"));
                    vec![relative_path.to_owned()]
                }
            }
            CanonicalTarget::None => Vec::new(),
        };
        entries.push(physical_regular_entry(
            reads,
            root,
            &path,
            PhysicalEntryDescriptor {
                stable_id: stable_id(binding.role, binding.relative_path),
                kind: kind(binding.role),
                owner: owner(binding.role),
                authority_state,
                active_status,
                generator: None,
                provenance,
                references,
            },
        )?);
    }
    Ok((entries, findings))
}

fn stable_id(role: BehavioralRole, path: &str) -> String {
    format!("BEHAVIORAL-ROLE:{}:{path}", kind(role).to_ascii_uppercase())
}

const fn kind(role: BehavioralRole) -> &'static str {
    match role {
        BehavioralRole::CliIngressAdapter => "cli-ingress-adapter",
        BehavioralRole::ClaimGuard => "claim-guard",
        BehavioralRole::TestContext => "test-context",
        BehavioralRole::PackageInputDescriptor => "package-input-descriptor",
    }
}

const fn owner(role: BehavioralRole) -> &'static str {
    match role {
        BehavioralRole::CliIngressAdapter => "OWN-CLI",
        BehavioralRole::ClaimGuard | BehavioralRole::TestContext => "OWN-PROOF-AUTHORITY",
        BehavioralRole::PackageInputDescriptor => "OWN-PLUGIN-PRODUCT",
    }
}
