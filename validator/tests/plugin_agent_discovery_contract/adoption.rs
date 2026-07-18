use super::super::AgentDiscoveryErrorId;
use super::super::local_authority::{
    AgentRepositoryAdoptionRequest, adopt_agent_repository, compatibility_observation_for_test,
};
use super::authority_fixtures::{CANDIDATE, SESSION, TempRepo, canonical_names};
use std::fs;
use std::path::Path;

#[test]
fn typed_repository_adoption_uses_one_explicit_authority_set() {
    let source = TempRepo::canonical();
    let home = source.root.join("host-home");
    let installed = home.join(".codex/plugins/harness-ultragoal");
    let cache_family = home.join(".codex/plugins/cache/local-harness-plugins/harness-ultragoal");
    let cache = cache_family.join("0.0.11");
    let project = source.root.join("project");
    copy_plugin_root(&source.root, &installed);
    copy_plugin_root(&source.root, &cache);
    copy_plugin_root(&source.root, &project);
    fs::create_dir_all(home.join(".codex/agents")).unwrap();

    let adoption = adopt_agent_repository(AgentRepositoryAdoptionRequest {
        source_root: &source.root,
        package_root: source.root.clone(),
        installed_root: installed,
        cache_family_root: cache_family,
        global_root: home,
        project_root: project,
        candidate_id: CANDIDATE,
        session_id: SESSION,
    })
    .unwrap();

    assert!(!adoption.fresh_session_observed());
    assert!(!adoption.route_eligible());
    assert_eq!(adoption.roles().len(), canonical_names().len());
    assert!(adoption.roles().iter().all(|role| {
        role.package_matches()
            && role.installed_matches()
            && role.cache_matches()
            && !role.global_matches()
            && role.project_matches()
    }));
}

#[test]
fn duplicate_package_and_project_authority_fails_closed() {
    let source = TempRepo::canonical();
    let home = source.root.join("host-home");
    let installed = home.join(".codex/plugins/harness-ultragoal");
    let cache_family = home.join(".codex/plugins/cache/local-harness-plugins/harness-ultragoal");
    copy_plugin_root(&source.root, &installed);
    copy_plugin_root(&source.root, &cache_family.join("0.0.11"));
    fs::create_dir_all(home.join(".codex/agents")).unwrap();

    let result = adopt_agent_repository(AgentRepositoryAdoptionRequest {
        source_root: &source.root,
        package_root: source.root.clone(),
        installed_root: installed,
        cache_family_root: cache_family,
        global_root: home,
        project_root: source.root.clone(),
        candidate_id: CANDIDATE,
        session_id: SESSION,
    });
    let Err(error) = result else {
        panic!("duplicate authority roots were accepted");
    };

    assert_eq!(error.id(), AgentDiscoveryErrorId::IdentityMismatch);
}

#[test]
fn compatibility_observation_rejects_fresh_session_or_route_eligible_results() {
    for (fresh_session, route_eligible) in [(true, false), (false, true), (true, true)] {
        assert_eq!(
            compatibility_observation_for_test(fresh_session, route_eligible)
                .unwrap_err()
                .id(),
            AgentDiscoveryErrorId::InvalidBinding
        );
    }
    compatibility_observation_for_test(false, false).unwrap();
}

fn copy_plugin_root(source: &Path, target: &Path) {
    fs::create_dir_all(target.join(".codex/agents")).unwrap();
    fs::create_dir_all(target.join(".codex-plugin")).unwrap();
    for name in canonical_names() {
        fs::copy(
            source.join(format!(".codex/agents/{name}.toml")),
            target.join(format!(".codex/agents/{name}.toml")),
        )
        .unwrap();
    }
    fs::copy(
        source.join(".codex-plugin/plugin.json"),
        target.join(".codex-plugin/plugin.json"),
    )
    .unwrap();
}
