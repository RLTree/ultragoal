use super::*;

#[test]
fn profile_errors_cover_missing_extra_duplicate_and_unknown_skills() {
    let make_profile = |selected_skills: Vec<&str>| SkillProfile {
        plugin: "agentic-engineering".to_owned(),
        name: "ultragoal".to_owned(),
        digest: PROFILE_DIGEST.to_owned(),
        candidate_id: CANDIDATE.to_owned(),
        config_digest: CONFIG.to_owned(),
        plugin_version: "3.0.0".to_owned(),
        plugin_digest: AGENTIC_PLUGIN_DIGEST.to_owned(),
        selected_skills: selected_skills.into_iter().map(str::to_owned).collect(),
    };
    let selected = package(
        "agentic-engineering",
        "3.0.0",
        AGENTIC_PLUGIN_DIGEST,
        AGENTIC_PACKAGE_DIGEST,
        vec![skill("ultragoal", false), skill("second", false)],
    );
    let harness = package(
        HARNESS_PLUGIN,
        "0.0.19",
        HARNESS_PLUGIN_DIGEST,
        HARNESS_PACKAGE_DIGEST,
        vec![skill(HARNESS_FRONT_DOOR, true)],
    );
    assert!(matches!(
        project(&request(
            vec![harness.clone(), selected.clone()],
            Some(make_profile(vec!["ultragoal"]))
        )),
        Err(SkillCatalogError::OmittedProfileSkill(_))
    ));
    assert!(matches!(
        project(&request(
            vec![harness.clone(), selected.clone()],
            Some(make_profile(vec!["ultragoal", "second", "extra"]))
        )),
        Err(SkillCatalogError::UnknownProfileSkill(_))
    ));
    assert!(matches!(
        project(&request(
            vec![harness.clone(), selected.clone()],
            Some(make_profile(vec!["ultragoal", "ultragoal"]))
        )),
        Err(SkillCatalogError::DuplicateProfileSkill(_))
    ));
    assert!(matches!(
        project(&request(
            vec![harness, selected],
            Some(make_profile(vec!["unknown", "second"]))
        )),
        Err(SkillCatalogError::UnknownProfileSkill(_))
    ));
}

#[test]
fn wrong_version_profile_digest_and_candidate_fail_closed() {
    let mut profile = SkillProfile {
        plugin: "agentic-engineering".to_owned(),
        name: "ultragoal".to_owned(),
        digest: PROFILE_DIGEST.to_owned(),
        candidate_id: CANDIDATE.to_owned(),
        config_digest: CONFIG.to_owned(),
        plugin_version: "wrong".to_owned(),
        plugin_digest: AGENTIC_PLUGIN_DIGEST.to_owned(),
        selected_skills: vec!["ultragoal".to_owned()],
    };
    let agentic = package(
        "agentic-engineering",
        "3.0.0",
        AGENTIC_PLUGIN_DIGEST,
        AGENTIC_PACKAGE_DIGEST,
        vec![skill("ultragoal", false)],
    );
    assert!(matches!(
        project(&request(vec![agentic.clone()], Some(profile.clone()))),
        Err(SkillCatalogError::StaleProfile(_))
    ));
    profile.plugin_version = "3.0.0".to_owned();
    profile.digest = "wrong".to_owned();
    assert!(matches!(
        project(&request(vec![agentic.clone()], Some(profile.clone()))),
        Err(SkillCatalogError::InvalidDigest("profile_digest"))
    ));
    profile.digest = PROFILE_DIGEST.to_owned();
    profile.candidate_id =
        "sha256:9999999999999999999999999999999999999999999999999999999999999999".to_owned();
    assert!(matches!(
        project(&request(vec![agentic], Some(profile))),
        Err(SkillCatalogError::CrossCandidate(_))
    ));
}

#[test]
fn non_harness_implicit_gateway_is_rejected() {
    let packages = vec![package(
        "agentic-engineering",
        "3.0.0",
        AGENTIC_PLUGIN_DIGEST,
        AGENTIC_PACKAGE_DIGEST,
        vec![skill("ultragoal", true)],
    )];
    assert!(matches!(
        project(&request(packages, None)),
        Err(SkillCatalogError::UnexpectedImplicitSkill(plugin)) if plugin == "agentic-engineering"
    ));
}

#[test]
fn legacy_aliases_are_excluded_and_warned() {
    let packages = vec![package(
        HARNESS_PLUGIN,
        "0.0.19",
        HARNESS_PLUGIN_DIGEST,
        HARNESS_PACKAGE_DIGEST,
        vec![skill(HARNESS_FRONT_DOOR, true), skill("ultragoal", false)],
    )];
    let projection = project(&request(packages, None)).expect("projection");
    assert_eq!(projection.enabled_skill_count, 1);
    assert!(projection.warnings.iter().any(|warning| matches!(
        warning,
        CatalogWarning::LegacyAliasExcluded { skill, .. } if skill == "ultragoal"
    )));
}

#[test]
fn omitted_and_truncated_metadata_are_visible_as_warnings() {
    let mut omitted = skill("prove", false);
    let mut truncated = skill("diagnose-and-observe", false);
    truncated.openai_yaml = yaml("diagnose-and-observe", "description...", false);
    omitted.enabled = false;
    let packages = vec![package(
        HARNESS_PLUGIN,
        "0.0.19",
        HARNESS_PLUGIN_DIGEST,
        HARNESS_PACKAGE_DIGEST,
        vec![skill(HARNESS_FRONT_DOOR, true), omitted, truncated],
    )];
    let projection = project(&request(packages, None)).expect("projection");
    assert!(projection.warnings.iter().any(
        |warning| matches!(warning, CatalogWarning::OmittedSkill { skill, .. } if skill == "prove")
    ));
    assert!(projection.warnings.iter().any(|warning| matches!(
        warning,
        CatalogWarning::TruncatedDescription { skill, .. } if skill == "diagnose-and-observe"
    )));
}

#[test]
fn unsupported_yaml_is_rejected_instead_of_approximated() {
    let source = "interface:\n  display_name: Front door\n  short_description: bounded\n  default_prompt: Use it.\n  extra: reject\npolicy:\n  allow_implicit_invocation: true\n";
    assert!(matches!(
        super::parser::parse_skill_metadata(source),
        Err(super::parser::YamlError::UnsupportedField(_))
    ));
}

#[test]
fn yaml_document_streams_are_rejected() {
    let source = "---\ninterface:\n  display_name: Front door\n  short_description: bounded\n  default_prompt: Use it.\npolicy:\n  allow_implicit_invocation: true\n...\ninterface:\n  display_name: Second\n";
    assert!(matches!(
        super::parser::parse_skill_metadata(source),
        Err(super::parser::YamlError::InvalidLine(_))
    ));
}

#[test]
fn folded_blocks_with_more_indented_content_are_rejected() {
    let source = "interface:\n  display_name: Front door\n  short_description: >-\n    First line\n      More-indented line\n  default_prompt: Use it.\npolicy:\n  allow_implicit_invocation: true\n";
    assert!(matches!(
        super::parser::parse_skill_metadata(source),
        Err(super::parser::YamlError::InvalidLine(_))
    ));
}

#[test]
fn noncanonical_skill_path_is_rejected() {
    let mut harness = package(
        HARNESS_PLUGIN,
        "0.0.19",
        HARNESS_PLUGIN_DIGEST,
        HARNESS_PACKAGE_DIGEST,
        vec![skill(HARNESS_FRONT_DOOR, true)],
    );
    harness.skills[0].path = "README.md".to_owned();
    assert!(matches!(
        project(&request(vec![harness], None)),
        Err(SkillCatalogError::InvalidPath(path)) if path == "README.md"
    ));
}
