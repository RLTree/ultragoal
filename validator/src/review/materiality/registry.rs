use crate::review::round::{anchor::refs::VerifiedRef, config::REVIEW_ROLES};

pub(super) fn registry_contents_valid(row: &VerifiedRef) -> bool {
    let manifests: [&[u8]; 4] = [
        include_bytes!("../../../../.codex/agents/claim-falsifier.toml"),
        include_bytes!("../../../../.codex/agents/orchestration-recovery-reviewer.toml"),
        include_bytes!("../../../../.codex/agents/security-reviewer.toml"),
        include_bytes!("../../../../.codex/agents/product-journey-reviewer.toml"),
    ];
    row.value.as_ref().is_some_and(|value| {
        value["status"] == "fail"
            && value["claim_ceiling"] == "withheld_or_blocked"
            && value["agent_types"].as_array().is_some_and(|rows| {
                rows.len() == REVIEW_ROLES.len()
                    && REVIEW_ROLES.iter().zip(manifests).all(|(spec, manifest)| {
                        rows.iter().any(|row| {
                            row["role"] == spec.role_name
                                && row["agent_manifest_path"] == spec.agent_manifest_path
                                && row["agent_manifest_digest"] == crate::digest::bytes(manifest)
                                && row["source_manifest_present"] == true
                                && row["exposed"] == false
                                && row["sandbox_mode"] == "read-only"
                                && row["runtime_metadata_status"] == "unavailable"
                                && row["custom_agent_discovery_status"] == "unavailable"
                        })
                    })
            })
    })
}
