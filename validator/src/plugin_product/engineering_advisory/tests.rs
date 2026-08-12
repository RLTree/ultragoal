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
        aggregate_digest: value.aggregate_digest.clone(),
        manifest_digests: value
            .packs
            .iter()
            .map(|pack| (pack.name.clone(), pack.manifest_digest.clone()))
            .collect(),
    }
}

#[test]
fn base_only_is_a_valid_explicit_pack_set() {
    let value = set(vec![pack("agentic-engineering", &BASE_SKILLS, 1)]);
    assert_eq!(value.validate_against(&binding(&value)), Ok(()));
    assert_eq!(
        value
            .select(&binding(&value), "agentic-engineering:codex-task-contract",)
            .unwrap(),
        EngineeringAdvisorySelection {
            qualified_skill: "agentic-engineering:codex-task-contract".to_owned(),
            pack_set_digest: value.aggregate_digest.clone(),
            proposal_only: true,
            claim_effect: "none",
        }
    );
}

#[test]
fn full_pack_set_has_the_exact_thirty_skill_union() {
    let value = set(vec![
        pack("agentic-engineering", &BASE_SKILLS, 1),
        pack("agentic-engineering-lifecycle", &LIFECYCLE_SKILLS, 2),
        pack("agentic-engineering-rust", &RUST_SKILLS, 3),
        pack("agentic-engineering-systems", &SYSTEMS_SKILLS, 4),
    ]);
    assert_eq!(value.validate_against(&binding(&value)), Ok(()));
    assert_eq!(
        value
            .select(
                &binding(&value),
                "agentic-engineering-rust:harness-engineering",
            )
            .unwrap()
            .pack_set_digest,
        value.aggregate_digest
    );
}

#[test]
fn missing_or_substituted_packs_fail_closed() {
    let value = set(vec![pack("agentic-engineering", &BASE_SKILLS, 1)]);
    let mut stale = binding(&value);
    stale.manifest_digests.insert(
        "agentic-engineering".to_owned(),
        "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_owned(),
    );
    assert_eq!(
        value.validate_against(&stale),
        Err(AdvisoryError::CandidateDigestMismatch)
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
    let mut duplicate = set(vec![pack("agentic-engineering", &BASE_SKILLS, 1)]);
    duplicate.packs[0]
        .enabled_skills
        .push(BASE_SKILLS[0].to_owned());
    assert_eq!(
        duplicate.validate_against(&binding(&duplicate)),
        Err(AdvisoryError::DuplicateSkill)
    );

    let mut omitted = set(vec![pack("agentic-engineering", &BASE_SKILLS, 1)]);
    omitted.packs[0].enabled_skills.pop();
    assert_eq!(
        omitted.validate_against(&binding(&omitted)),
        Err(AdvisoryError::SkillSetMismatch)
    );

    let mut second_gateway = set(vec![pack("agentic-engineering", &BASE_SKILLS, 1)]);
    second_gateway.gateway = "external:other".to_owned();
    assert_eq!(
        second_gateway.validate_against(&binding(&second_gateway)),
        Err(AdvisoryError::SecondGateway)
    );
}
