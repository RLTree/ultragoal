use super::*;
use std::collections::BTreeSet;

#[test]
fn stage_profiles_cover_every_agentic_skill_without_a_second_gateway() {
    let profiles = [
        coinstall_profile(AgenticAdvisoryStage::UltraGoal, CANDIDATE, CONFIG),
        coinstall_profile(AgenticAdvisoryStage::Core, CANDIDATE, CONFIG),
        coinstall_profile(AgenticAdvisoryStage::Lifecycle, CANDIDATE, CONFIG),
        coinstall_profile(AgenticAdvisoryStage::Rust, CANDIDATE, CONFIG),
    ];
    let covered = profiles
        .iter()
        .flat_map(|profile| profile.profile.selected_skills.iter().cloned())
        .collect::<BTreeSet<_>>();
    let expected = all_agentic_skills()
        .iter()
        .map(|skill| (*skill).to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(covered, expected);
    assert!(covered.contains("agentic-engineering"));
    assert!(covered.contains("product-fitness-engineering"));
    assert!(covered.contains("rust-agent-durability"));
    assert_eq!(
        profiles[0].source_implicit_front_door,
        EXTERNAL_HARNESS_GATEWAY
    );
    assert!(
        profiles[1..]
            .iter()
            .all(|profile| profile.source_implicit_front_door == AGENTIC_PLUGIN)
    );
    assert!(profiles.iter().all(|profile| {
        profile.external_front_door == EXTERNAL_HARNESS_GATEWAY
            && profile.profile.plugin == AGENTIC_PLUGIN
            && profile.profile.plugin_version == AGENTIC_PLUGIN_VERSION
            && profile.profile.plugin_digest == AGENTIC_FILE_MANIFEST_DIGEST
    }));
}

#[test]
fn core_profile_projects_under_the_harness_gateway_with_budget_margin() {
    let profile = coinstall_profile(AgenticAdvisoryStage::Core, CANDIDATE, CONFIG);
    let selected = profile
        .profile
        .selected_skills
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let agentic_skills = [
        "agentic-engineering",
        "codex-task-contract",
        "context-repository-engineering",
        "harness-engineering",
        "loop-engineering",
        "graph-workflow-engineering",
        "multi-agent-engineering",
        "agent-evals-observability",
        "agent-security-governance",
        "agent-first-software-engineering",
        "engineering-learning-loop",
        "verification-strategy-engineering",
        "product-fitness-engineering",
    ]
    .into_iter()
    .map(|name| {
        let mut artifact = skill(name, false);
        artifact.enabled = selected.contains(name);
        artifact
    })
    .collect::<Vec<_>>();
    let catalog_request = request(
        vec![
            package(
                HARNESS_PLUGIN,
                "0.0.23",
                HARNESS_PLUGIN_DIGEST,
                HARNESS_PACKAGE_DIGEST,
                vec![skill(HARNESS_FRONT_DOOR, true)],
            ),
            package(
                AGENTIC_PLUGIN,
                AGENTIC_PLUGIN_VERSION,
                AGENTIC_FILE_MANIFEST_DIGEST,
                AGENTIC_PACKAGE_DIGEST,
                agentic_skills,
            ),
        ],
        Some(profile.profile.clone()),
    );
    let projection = project_coinstall(&catalog_request, &profile).expect("co-install projection");

    assert_eq!(projection.implicit_gateways, vec![HARNESS_PLUGIN]);
    assert_eq!(projection.enabled_skill_count, 13);
    assert!(projection.headroom > 0);
}

#[test]
fn coinstall_rejects_substituted_profile_and_second_implicit_gateway() {
    let profile = coinstall_profile(AgenticAdvisoryStage::UltraGoal, CANDIDATE, CONFIG);
    let packages = vec![
        package(
            HARNESS_PLUGIN,
            "0.0.23",
            HARNESS_PLUGIN_DIGEST,
            HARNESS_PACKAGE_DIGEST,
            vec![skill(HARNESS_FRONT_DOOR, true)],
        ),
        package(
            AGENTIC_PLUGIN,
            AGENTIC_PLUGIN_VERSION,
            AGENTIC_FILE_MANIFEST_DIGEST,
            AGENTIC_PACKAGE_DIGEST,
            profile
                .profile
                .selected_skills
                .iter()
                .enumerate()
                .map(|(index, name)| skill(name, index == 0))
                .collect(),
        ),
    ];
    let request = request(packages, Some(profile.profile.clone()));
    assert!(matches!(
        project_coinstall(&request, &profile),
        Err(SkillCatalogError::MultipleImplicitGateways(_))
    ));

    let mut substituted = profile.clone();
    substituted.profile.config_digest = PROFILE_DIGEST.to_owned();
    assert!(matches!(
        project_coinstall(&request, &substituted),
        Err(SkillCatalogError::StaleProfile("agentic_profile_config"))
    ));
}
