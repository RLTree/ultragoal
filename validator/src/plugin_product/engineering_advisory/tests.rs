use super::*;

fn pack(name: &str, skills: &[&str], index: u8) -> AgenticPackV1 {
    AgenticPackV1 {
        name: name.to_owned(),
        version: AGENTIC_VERSION.to_owned(),
        manifest_digest: format!("sha256:{index:064x}"),
        enabled_skills: skills.iter().map(|value| (*value).to_owned()).collect(),
    }
}

fn set(packs: Vec<AgenticPackV1>) -> AgenticPackSetV1 {
    let aggregate_digest = computed_digest(&packs).unwrap();
    AgenticPackSetV1 {
        schema_version: AGENTIC_PACK_SET_SCHEMA.to_owned(),
        gateway: AGENTIC_GATEWAY.to_owned(),
        aggregate_digest,
        packs,
    }
}

fn binding(value: &AgenticPackSetV1) -> AgenticCandidateBinding {
    AgenticCandidateBinding {
        source_commit: AGENTIC_PACK_SOURCE_COMMIT.to_owned(),
        source_tree: AGENTIC_PACK_SOURCE_TREE.to_owned(),
        aggregate_digest: value.aggregate_digest.clone(),
        manifest_digests: value
            .packs
            .iter()
            .map(|pack| (pack.name.clone(), pack.manifest_digest.clone()))
            .collect(),
        package_digests: exact_agentic_package_digests(),
    }
}

#[test]
fn full_pack_set_has_the_exact_thirty_skill_union() {
    let value = exact_agentic_pack_set();
    assert_eq!(
        value.validate_against(&exact_agentic_candidate_binding()),
        Ok(())
    );
    assert_eq!(
        value
            .select(
                &exact_agentic_candidate_binding(),
                "agentic-engineering-rust:harness-engineering",
            )
            .unwrap()
            .pack_set_digest,
        value.aggregate_digest
    );
}

#[test]
fn exact_candidate_is_the_only_dependency_closed_identity() {
    let value = exact_agentic_pack_set();
    let candidate = exact_agentic_candidate_binding();
    assert_eq!(value.aggregate_digest, AGENTIC_PACK_SET_DIGEST);
    assert_eq!(AGENTIC_PACK_SOURCE_COMMIT.len(), 40);
    assert_eq!(AGENTIC_PACK_SOURCE_TREE.len(), 40);
    assert_eq!(
        computed_digest(&value.packs).unwrap(),
        AGENTIC_PACK_SET_DIGEST
    );
    assert_eq!(value.validate_against(&candidate), Ok(()));
    assert_eq!(
        value
            .select(&candidate, "agentic-engineering:codex-task-contract")
            .unwrap(),
        EngineeringAdvisorySelection {
            qualified_skill: "agentic-engineering:codex-task-contract".to_owned(),
            pack_set_digest: AGENTIC_PACK_SET_DIGEST.to_owned(),
            proposal_only: true,
            claim_effect: "none",
        }
    );
    assert_eq!(
        qualify_skill("rust-agent-durability").as_deref(),
        Some("agentic-engineering-rust:rust-agent-durability")
    );
}

#[test]
fn missing_or_substituted_packs_fail_closed() {
    let value = set(vec![pack("agentic-engineering", &BASE_SKILLS, 1)]);
    assert_eq!(
        value.validate_against(&binding(&value)),
        Err(AdvisoryError::CompanionPackRequired)
    );
    let mut stale = binding(&value);
    stale.manifest_digests.insert(
        "agentic-engineering".to_owned(),
        "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_owned(),
    );
    assert_eq!(
        value.validate_against(&stale),
        Err(AdvisoryError::CompanionPackRequired)
    );
    assert_eq!(
        value.select(
            &binding(&value),
            "agentic-engineering-rust:harness-engineering",
        ),
        Err(AdvisoryError::AdviceUnavailable)
    );
}

#[test]
fn legacy_monolith_and_same_version_source_cache_mismatch_are_rejected() {
    let mut legacy = exact_agentic_pack_set();
    legacy.packs[0].version = "3.0.1".to_owned();
    assert_eq!(
        legacy.validate_against(&exact_agentic_candidate_binding()),
        Err(AdvisoryError::InvalidVersion)
    );

    let mut mutable_source = exact_agentic_pack_set();
    mutable_source.packs[0].manifest_digest =
        "sha256:23bae48d6063fe876fc644e0f1b25b34964b591c149bfc7cf0e8b3e81fa4113a".to_owned();
    mutable_source.aggregate_digest = computed_digest(&mutable_source.packs).unwrap();
    assert_eq!(
        mutable_source.validate_against(&exact_agentic_candidate_binding()),
        Err(AdvisoryError::CandidateDigestMismatch)
    );

    let exact = exact_agentic_pack_set();
    let mut mutable_source_provenance = exact_agentic_candidate_binding();
    mutable_source_provenance.source_commit = "9861df7c7894b35f5ce758ee1b005f80ceb0426e".to_owned();
    assert_eq!(
        exact.validate_against(&mutable_source_provenance),
        Err(AdvisoryError::CandidateDigestMismatch)
    );

    let mut mutable_tree_provenance = exact_agentic_candidate_binding();
    mutable_tree_provenance.source_tree = "b".repeat(40);
    assert_eq!(
        exact.validate_against(&mutable_tree_provenance),
        Err(AdvisoryError::CandidateDigestMismatch)
    );

    let mut mutable_package_bytes = exact_agentic_candidate_binding();
    *mutable_package_bytes
        .package_digests
        .get_mut("agentic-engineering")
        .expect("base package") =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned();
    assert_eq!(
        exact.validate_against(&mutable_package_bytes),
        Err(AdvisoryError::CandidateDigestMismatch)
    );
}

#[test]
fn digest_normalizes_enabled_skill_order() {
    let forward = set(vec![pack("agentic-engineering", &BASE_SKILLS, 1)]);
    let mut reversed = forward.packs.clone();
    reversed[0].enabled_skills.reverse();
    assert_eq!(
        forward.aggregate_digest,
        computed_digest(&reversed).unwrap()
    );
}

#[test]
fn duplicate_or_omitted_skills_and_second_gateway_are_rejected() {
    let exact = exact_agentic_pack_set();
    let mut duplicate_pack = exact.clone();
    duplicate_pack.packs[3] = duplicate_pack.packs[0].clone();
    duplicate_pack.aggregate_digest = computed_digest(&duplicate_pack.packs).unwrap();
    assert_eq!(
        duplicate_pack.validate_against(&binding(&duplicate_pack)),
        Err(AdvisoryError::DuplicatePack)
    );

    let mut duplicate = exact_agentic_pack_set();
    duplicate.packs[0]
        .enabled_skills
        .push(BASE_SKILLS[0].to_owned());
    assert_eq!(
        duplicate.validate_against(&binding(&duplicate)),
        Err(AdvisoryError::DuplicateSkill)
    );

    let mut omitted = exact_agentic_pack_set();
    omitted.packs[0].enabled_skills.pop();
    assert_eq!(
        omitted.validate_against(&binding(&omitted)),
        Err(AdvisoryError::SkillSetMismatch)
    );

    let mut second_gateway = exact_agentic_pack_set();
    second_gateway.gateway = "external:other".to_owned();
    assert_eq!(
        second_gateway.validate_against(&binding(&second_gateway)),
        Err(AdvisoryError::SecondGateway)
    );
}
