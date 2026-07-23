use crate::distribution::canonical_skill_names;
use crate::inventory::compatibility::READER_PROOF_SHA256;
use crate::plugin_product::skill_catalog::legacy_aliases;

fn validate_skill_authority_inputs(
    family: &MigrationAdoptionFamily,
    surfaces: &BTreeMap<&str, &InventorySurface>,
    input: &ProductInputSnapshot,
) -> bool {
    const MANIFEST_ID: &str = "BEHAVIORAL-ROLE:PACKAGE-INPUT-DESCRIPTOR:plugin-manifest-draft.json";
    const SCHEMA_ID: &str = "SCHEMA:schemas/plugin-manifest.schema.json";
    let Some(manifest) = surfaces.get(MANIFEST_ID).copied() else {
        return false;
    };
    let Some(schema) = surfaces.get(SCHEMA_ID).copied() else {
        return false;
    };
    let Some(activation) = input.skill_activation() else {
        return false;
    };
    let expected_package_exclusion = digest(
        format!(
            "n14-skill-package-manifest-exclusion-v3|{}|{}|{}|{}",
            manifest.digest_sha256,
            schema.digest_sha256,
            canonical_skill_names().join(","),
            legacy_aliases().join(","),
        )
        .as_bytes(),
    );
    let result = manifest.kind == "package-input-descriptor"
        && manifest.relative_path == "plugin-manifest-draft.json"
        && manifest.status == SurfaceStatus::Active
        && manifest.file_kind == crate::migration::SurfaceFileKind::Semantic
        && manifest.link_count == 0
        && schema.kind == "schema"
        && schema.relative_path == "schemas/plugin-manifest.schema.json"
        && schema.status == SurfaceStatus::ContextOnly
        && schema.file_kind == crate::migration::SurfaceFileKind::Semantic
        && schema.link_count == 0
        && family.skill_package_manifest_surface_id == MANIFEST_ID
        && family.skill_package_manifest_sha256 == manifest.digest_sha256
        && family.skill_package_schema_surface_id == SCHEMA_ID
        && family.skill_package_schema_sha256 == schema.digest_sha256
        && activation.validate()
        && activation.candidate_id() == input.inventory().candidate_id()
        && activation.catalog_id() == input.inventory().catalog_id()
        && activation.canonical_skill_names()
            == canonical_skill_names()
                .iter()
                .map(|name| (*name).to_owned())
                .collect::<Vec<_>>()
        && activation.excluded_legacy_names()
            == legacy_aliases()
                .iter()
                .map(|name| (*name).to_owned())
                .collect::<Vec<_>>()
        && activation.profile_rejected_legacy()
        && activation.implicit_gateways() == ["harness-ultragoal".to_owned()]
        && family.skill_no_catalog_evidence_id == "N14-CURRENT-CATALOG-EXCLUSION-PROJECTION"
        && family.skill_no_profile_evidence_id == "N14-CURRENT-PROFILE-REJECTION-PROJECTION"
        && family.skill_implicit_gateway_evidence_id == "N14-SOLE-IMPLICIT-GATEWAY-PROJECTION"
        && family.skill_no_package_evidence_sha256 == expected_package_exclusion;
    result
}

fn valid_family_hashes(family: &MigrationAdoptionFamily) -> bool {
    let agent_routes = agent_routes()
        .map(route_fragment)
        .collect::<Vec<_>>()
        .join("|");
    let skill_route_fragments = skill_routes()
        .map(skill_route_fragment)
        .collect::<Vec<_>>()
        .join("|");
    let agent_routes_sha = digest(agent_routes.as_bytes());
    let skill_routes_sha = digest(skill_route_fragments.as_bytes());
    let expected_agent_evidence = digest(
        format!(
            "n14-agent-context-evidence-v1|{}|{}|{}|{}|{}",
            prefixed(READER_PROOF_SHA256),
            digest(b"n14-agent-no-discovery-v1"),
            digest(b"n14-agent-no-package-v1"),
            digest(b"n14-agent-no-public-route-v1"),
            agent_routes_sha,
        )
        .as_bytes(),
    );
    let expected_skill_wrapper =
        digest(format!("n14-skill-wrapper-family-v1|{skill_routes_sha}").as_bytes());
    let expected_family_rollback = digest(
        format!(
            "n14-family-rollback-v1|{}|{}",
            family.agent_rollback_execution_sha256, family.skill_rollback_execution_sha256
        )
        .as_bytes(),
    );
    family.agent_behavior_execution_kind == AGENT_BEHAVIOR_KIND
        && family.skill_behavior_execution_kind == SKILL_BEHAVIOR_KIND
        && family.agent_behavior_execution_sha256 == digest(b"n14-agent-context-behavior-v1")
        && family.skill_behavior_execution_sha256 == digest(b"n14-skill-wrapper-forward-v1")
        && family.agent_rollback_execution_sha256 == digest(b"n14-agent-context-rollback-v1")
        && family.skill_rollback_execution_sha256 == digest(b"n14-skill-wrapper-rollback-v1")
        && family.agent_reader_evidence_sha256 == prefixed(READER_PROOF_SHA256)
        && family.agent_no_discovery_evidence_sha256 == digest(b"n14-agent-no-discovery-v1")
        && family.agent_no_package_evidence_sha256 == digest(b"n14-agent-no-package-v1")
        && family.agent_no_public_route_evidence_sha256 == digest(b"n14-agent-no-public-route-v1")
        && family.agent_evidence_sha256 == expected_agent_evidence
        && family.skill_wrapper_family_sha256 == expected_skill_wrapper
        && family.skill_implicit_gateway_target == "SKILL:harness-ultragoal"
        && skill_routes().all(|route| route.canonical_name != "agentic-engineering")
        && family.family_rollback_execution_sha256 == expected_family_rollback
        && family.false_pass_control_sha256.len() == REQUIRED_CONTROLS.len()
        && REQUIRED_CONTROLS.iter().all(|control| {
            family.false_pass_control_sha256.get(*control)
                == Some(&digest(
                    format!("n14-family-false-pass-v1|{control}").as_bytes(),
                ))
        })
}
