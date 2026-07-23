use super::*;

const CANDIDATE: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CONFIG: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HARNESS_PLUGIN_DIGEST: &str =
    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HARNESS_PACKAGE_DIGEST: &str =
    "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const PROFILE_DIGEST: &str =
    "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const AGENTIC_PLUGIN_DIGEST: &str =
    "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const AGENTIC_PACKAGE_DIGEST: &str =
    "sha256:1111111111111111111111111111111111111111111111111111111111111111";

fn yaml(name: &str, short: &str, implicit: bool) -> String {
    format!(
        "interface:\n  display_name: {name}\n  short_description: {short}\n  default_prompt: Use {name}.\npolicy:\n  allow_implicit_invocation: {implicit}\n"
    )
}

fn package(
    plugin: &str,
    version: &str,
    plugin_digest: &str,
    package_digest: &str,
    skills: Vec<SkillArtifact>,
) -> SkillPackage {
    SkillPackage {
        plugin: plugin.to_owned(),
        version: version.to_owned(),
        plugin_digest: plugin_digest.to_owned(),
        package_digest: package_digest.to_owned(),
        candidate_id: CANDIDATE.to_owned(),
        config_digest: CONFIG.to_owned(),
        selected: true,
        skills,
    }
}

fn skill(name: &str, implicit: bool) -> SkillArtifact {
    SkillArtifact {
        name: name.to_owned(),
        path: format!("skills/{name}/agents/openai.yaml"),
        openai_yaml: yaml(name, "bounded description", implicit),
        enabled: true,
    }
}

fn request(packages: Vec<SkillPackage>, profile: Option<SkillProfile>) -> SkillCatalogRequest {
    SkillCatalogRequest {
        candidate_id: CANDIDATE.to_owned(),
        config_digest: CONFIG.to_owned(),
        profile,
        packages,
    }
}

#[test]
fn parses_multiline_yaml_without_truncating_description() {
    let source = "interface:\n  display_name: Front door\n  short_description: |-\n    First line\n    Second line\n  default_prompt: >-\n    Use the front door\n    for one route.\npolicy:\n  allow_implicit_invocation: true\n";
    let parsed = super::parser::parse_skill_metadata(source).expect("yaml");
    assert_eq!(parsed.short_description, "First line\nSecond line");
    assert_eq!(parsed.default_prompt, "Use the front door for one route.");
}

#[test]
fn harness_only_topology_has_one_gateway_and_read_claim_ceiling() {
    let packages = vec![package(
        HARNESS_PLUGIN,
        "0.0.17",
        HARNESS_PLUGIN_DIGEST,
        HARNESS_PACKAGE_DIGEST,
        vec![skill(HARNESS_FRONT_DOOR, true), skill("prove", false)],
    )];
    let projection = project(&request(packages, None)).expect("projection");
    assert_eq!(projection.enabled_skill_count, 2);
    assert_eq!(projection.implicit_gateways, vec![HARNESS_PLUGIN]);
    assert!(projection.headroom > 0);
    assert_eq!(projection.effect, CatalogEffect::Read);
    assert!(!projection.claim_effect);
}

#[test]
fn two_implicit_gateways_fail_closed() {
    let packages = vec![
        package(
            HARNESS_PLUGIN,
            "0.0.17",
            HARNESS_PLUGIN_DIGEST,
            HARNESS_PACKAGE_DIGEST,
            vec![skill(HARNESS_FRONT_DOOR, true)],
        ),
        package(
            "agentic-engineering",
            "3.0.0",
            AGENTIC_PLUGIN_DIGEST,
            AGENTIC_PACKAGE_DIGEST,
            vec![skill("ultragoal", true)],
        ),
    ];
    assert!(matches!(
        project(&request(packages, None)),
        Err(SkillCatalogError::MultipleImplicitGateways(_))
    ));
}

#[test]
fn agentic_profile_plus_harness_has_one_gateway_and_margin() {
    let profile = SkillProfile {
        plugin: "agentic-engineering".to_owned(),
        name: "ultragoal".to_owned(),
        digest: PROFILE_DIGEST.to_owned(),
        candidate_id: CANDIDATE.to_owned(),
        config_digest: CONFIG.to_owned(),
        plugin_version: "3.0.0".to_owned(),
        plugin_digest: AGENTIC_PLUGIN_DIGEST.to_owned(),
        selected_skills: vec!["ultragoal".to_owned()],
    };
    let mut unused = skill("unused", false);
    unused.enabled = false;
    let packages = vec![
        package(
            HARNESS_PLUGIN,
            "0.0.17",
            HARNESS_PLUGIN_DIGEST,
            HARNESS_PACKAGE_DIGEST,
            vec![skill(HARNESS_FRONT_DOOR, true)],
        ),
        package(
            "agentic-engineering",
            "3.0.0",
            AGENTIC_PLUGIN_DIGEST,
            AGENTIC_PACKAGE_DIGEST,
            vec![skill("ultragoal", false), unused],
        ),
    ];
    let projection = project(&request(packages, Some(profile))).expect("projection");
    assert_eq!(projection.enabled_skill_count, 2);
    assert_eq!(projection.implicit_gateways, vec![HARNESS_PLUGIN]);
    assert!(projection.headroom > 0);
}

#[path = "tests_profile.rs"]
mod profile_tests;
