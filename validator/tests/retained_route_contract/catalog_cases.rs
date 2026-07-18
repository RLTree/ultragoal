use super::super::{
    AUTHORITY_CLASSIFICATION, RESULT_PATH, ROUTE_COUNT, TARGET_COUNT, by_route_id, by_stable_id,
    registry_route_is_compiled, routes, targets,
};
use super::route_transition_fixture::{Fixture, canonical_definition_sha256, repo_root, sha256};
use std::collections::BTreeSet;

#[test]
fn all_thirty_three_specs_are_unique_exact_and_bound_to_current_bytes() {
    let specs = routes().collect::<Vec<_>>();
    assert_eq!(specs.len(), ROUTE_COUNT);
    assert_eq!(targets().count(), TARGET_COUNT);
    let mut route_ids = BTreeSet::new();
    let mut stable_ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut counts = [0_usize; 4];

    for spec in specs {
        assert!(route_ids.insert(spec.route_id));
        assert!(stable_ids.insert(spec.stable_id));
        assert!(paths.insert(spec.path));
        assert!(spec.route_id.len() <= 96);
        assert!(
            spec.route_id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        );
        assert_eq!(by_route_id(spec.route_id), Some(spec));
        assert_eq!(by_stable_id(spec.stable_id), Some(spec));
        assert_eq!(spec.proof_refs, [spec.path, spec.target.path, RESULT_PATH]);
        assert_eq!(sha256(&repo_root().join(spec.path)), spec.source_sha256);
        let (index, stable_prefix, target_id) = match spec.kind {
            "legacy-command-authority" => (0, "LEGACY-COMMAND:", "PS-CLI"),
            "legacy-lane-authority" => (1, "LEGACY-LANE:", "PS-ORCHESTRATION"),
            "legacy-finalizer-authority" => (2, "LEGACY-FINALIZER:", "HCT-CLAIMS"),
            "legacy-manifest-projection-authority" => {
                (3, "LEGACY-MANIFEST-PROJECTION:", "PS-PLUGIN-MANIFEST")
            }
            other => panic!("unexpected pending authority kind: {other}"),
        };
        assert_eq!(spec.stable_id, format!("{stable_prefix}{}", spec.path));
        assert_eq!(spec.target.stable_id, target_id);
        counts[index] += 1;
    }
    assert_eq!(counts, [6, 14, 12, 1]);
}

#[test]
fn canonical_target_identity_is_the_exact_canonical_definition() {
    for target in targets() {
        assert_eq!(canonical_definition_sha256(target.stable_id), target.sha256);
        assert!(target.path.contains(&format!("/{}", target.stable_id)));
    }
}

#[test]
fn baseline_classifies_active_authority_without_demoting_it() {
    let fixture = Fixture::baseline();
    let sources_before = fixture.sources.clone();
    let result = fixture.verify().unwrap();
    assert_eq!(result.routes.len(), ROUTE_COUNT);
    assert_eq!(fixture.sources, sources_before);
    for verified in result.routes {
        assert_eq!(verified.classification(), AUTHORITY_CLASSIFICATION);
        assert!(verified.reclassifies_parallel_authority());
        assert!(registry_route_is_compiled(
            fixture
                .registry
                .iter()
                .find(|route| route.route_id == verified.spec.route_id)
                .unwrap()
        ));
    }
}
