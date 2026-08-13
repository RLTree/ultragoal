use super::*;
use crate::plugin_product::engineering_advisory::{
    AGENTIC_PACK_SET_DIGEST, exact_agentic_candidate_binding, exact_agentic_pack_set, qualify_skill,
};
use std::collections::BTreeSet;

fn agentic_packages(profile: &AgenticCoInstallProfile) -> Vec<SkillPackage> {
    let selected = profile
        .selected_skills
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let candidate = exact_agentic_candidate_binding();
    exact_agentic_pack_set()
        .packs
        .into_iter()
        .map(|pack_set| {
            let plugin = pack_set.name.clone();
            package(
                &plugin,
                &pack_set.version,
                &pack_set.manifest_digest,
                candidate
                    .package_digests
                    .get(&plugin)
                    .expect("exact package digest"),
                pack_set
                    .enabled_skills
                    .iter()
                    .map(|name| {
                        let mut artifact = skill(name, false);
                        artifact.enabled = selected.contains(&format!("{plugin}:{name}"));
                        artifact
                    })
                    .collect(),
            )
        })
        .collect()
}

fn harness_package() -> SkillPackage {
    package(
        HARNESS_PLUGIN,
        "0.0.28",
        HARNESS_PLUGIN_DIGEST,
        HARNESS_PACKAGE_DIGEST,
        vec![skill(HARNESS_FRONT_DOOR, true)],
    )
}

#[test]
fn stage_profiles_cover_every_agentic_skill_with_exact_four_pack_ownership() {
    let profiles = [
        coinstall_profile(AgenticAdvisoryStage::UltraGoal, CANDIDATE, CONFIG),
        coinstall_profile(AgenticAdvisoryStage::Core, CANDIDATE, CONFIG),
        coinstall_profile(AgenticAdvisoryStage::Lifecycle, CANDIDATE, CONFIG),
        coinstall_profile(AgenticAdvisoryStage::Rust, CANDIDATE, CONFIG),
    ];
    let covered = profiles
        .iter()
        .flat_map(|profile| profile.selected_skills.iter().cloned())
        .collect::<BTreeSet<_>>();
    let expected = all_agentic_skills()
        .iter()
        .map(|skill| qualify_skill(skill).expect("owned skill"))
        .collect::<BTreeSet<_>>();

    assert_eq!(covered, expected);
    assert!(covered.contains("agentic-engineering-lifecycle:agentic-engineering"));
    assert!(covered.contains("agentic-engineering:product-fitness-engineering"));
    assert!(covered.contains("agentic-engineering-rust:rust-agent-durability"));
    assert!(profiles.iter().all(|profile| {
        profile.schema_version == AGENTIC_COINSTALL_PROFILE_SCHEMA
            && profile.source_implicit_front_door == EXTERNAL_HARNESS_GATEWAY
            && profile.external_front_door == EXTERNAL_HARNESS_GATEWAY
            && profile.pack_set.aggregate_digest == AGENTIC_PACK_SET_DIGEST
            && profile.pack_set.packs.len() == 4
            && profile.candidate_binding == exact_agentic_candidate_binding()
    }));
}

#[test]
fn core_profile_projects_the_dependency_closed_pack_set_under_one_gateway() {
    let profile = coinstall_profile(AgenticAdvisoryStage::Core, CANDIDATE, CONFIG);
    let mut packages = vec![harness_package()];
    packages.extend(agentic_packages(&profile));
    let projection = project_coinstall(&request(packages, None), &profile).expect("co-install");

    assert_eq!(projection.implicit_gateways, vec![HARNESS_PLUGIN]);
    assert_eq!(projection.enabled_skill_count, 13);
    assert_eq!(
        projection
            .plugins
            .iter()
            .filter(|plugin| plugin.plugin.starts_with("agentic-engineering"))
            .count(),
        4
    );
    assert!(projection.profile.is_none());
    assert!(projection.headroom > 0);
}

#[test]
fn coinstall_rejects_missing_wrong_or_legacy_pack_identity() {
    let profile = coinstall_profile(AgenticAdvisoryStage::UltraGoal, CANDIDATE, CONFIG);
    let mut packages = vec![harness_package()];
    packages.extend(agentic_packages(&profile));

    let missing = packages
        .iter()
        .filter(|package| package.plugin != "agentic-engineering-rust")
        .cloned()
        .collect();
    assert!(matches!(
        project_coinstall(&request(missing, None), &profile),
        Err(SkillCatalogError::ProfilePluginMissing(name)) if name == "agentic-engineering-rust"
    ));

    packages
        .iter_mut()
        .find(|package| package.plugin == AGENTIC_PLUGIN)
        .expect("base pack")
        .plugin_digest =
        "sha256:23bae48d6063fe876fc644e0f1b25b34964b591c149bfc7cf0e8b3e81fa4113a".to_owned();
    assert!(matches!(
        project_coinstall(&request(packages.clone(), None), &profile),
        Err(SkillCatalogError::StaleProfile("agentic_pack_identity"))
    ));

    let mut wrong_package_digest = vec![harness_package()];
    wrong_package_digest.extend(agentic_packages(&profile));
    wrong_package_digest
        .iter_mut()
        .find(|package| package.plugin == AGENTIC_PLUGIN)
        .expect("base pack")
        .package_digest = AGENTIC_PACKAGE_DIGEST.to_owned();
    assert!(matches!(
        project_coinstall(&request(wrong_package_digest, None), &profile),
        Err(SkillCatalogError::StaleProfile("agentic_pack_identity"))
    ));

    let mut duplicate_packages = vec![harness_package()];
    duplicate_packages.extend(agentic_packages(&profile));
    duplicate_packages.push(
        duplicate_packages
            .iter()
            .find(|package| package.plugin == AGENTIC_PLUGIN)
            .expect("base pack")
            .clone(),
    );
    assert!(matches!(
        project_coinstall(&request(duplicate_packages, None), &profile),
        Err(SkillCatalogError::DuplicatePackage(name)) if name == AGENTIC_PLUGIN
    ));

    let mut extra_packages = vec![harness_package()];
    extra_packages.extend(agentic_packages(&profile));
    extra_packages.push(package(
        "agentic-engineering-unowned",
        "4.0.0",
        AGENTIC_PACKAGE_DIGEST,
        AGENTIC_PACKAGE_DIGEST,
        vec![skill("unowned", false)],
    ));
    assert!(matches!(
        project_coinstall(&request(extra_packages, None), &profile),
        Err(SkillCatalogError::StaleProfile("unexpected_agentic_pack"))
    ));

    let mut duplicate_skill_packages = vec![harness_package()];
    duplicate_skill_packages.extend(agentic_packages(&profile));
    let systems = duplicate_skill_packages
        .iter_mut()
        .find(|package| package.plugin == "agentic-engineering-systems")
        .expect("systems pack");
    let duplicate_skill = systems.skills.first().expect("systems skill").clone();
    systems.skills.push(duplicate_skill);
    assert!(matches!(
        project_coinstall(&request(duplicate_skill_packages, None), &profile),
        Err(SkillCatalogError::StaleProfile("agentic_pack_skills"))
    ));

    let legacy = SkillProfile {
        plugin: AGENTIC_PLUGIN.to_owned(),
        name: "legacy-monolith".to_owned(),
        digest: PROFILE_DIGEST.to_owned(),
        candidate_id: CANDIDATE.to_owned(),
        config_digest: CONFIG.to_owned(),
        plugin_version: "3.0.1".to_owned(),
        plugin_digest: "sha256:b0cf70a7db8fe86964acac725ac1a97502edf9369a8b3e8ce23b676ac78260fe"
            .to_owned(),
        selected_skills: vec!["codex-task-contract".to_owned()],
    };
    assert!(matches!(
        project_coinstall(&request(packages, Some(legacy)), &profile),
        Err(SkillCatalogError::StaleProfile(
            "legacy_single_plugin_profile"
        ))
    ));
}

#[test]
fn coinstall_rejects_a_second_implicit_gateway() {
    let profile = coinstall_profile(AgenticAdvisoryStage::UltraGoal, CANDIDATE, CONFIG);
    let mut packages = vec![harness_package()];
    packages.extend(agentic_packages(&profile));
    let agentic = packages
        .iter_mut()
        .find(|package| package.plugin == AGENTIC_PLUGIN)
        .expect("base pack");
    let selected = agentic
        .skills
        .iter_mut()
        .find(|skill| skill.enabled)
        .expect("selected skill");
    selected.openai_yaml = yaml(&selected.name, "bounded description", true);

    assert!(matches!(
        project_coinstall(&request(packages, None), &profile),
        Err(SkillCatalogError::MultipleImplicitGateways(_))
    ));
}
