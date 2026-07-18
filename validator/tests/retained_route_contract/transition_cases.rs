use crate::retained_routes::{RegistryRouteEvidence, VerificationError};
use crate::route_transition_fixture::Fixture;

#[test]
fn every_transition_overclaim_fails_closed() {
    let baseline = Fixture::baseline().registry[0];
    let mut transitions = Vec::new();
    macro_rules! changed {
        ($field:ident, $value:expr) => {{
            let mut value = baseline.transition;
            value.$field = $value;
            transitions.push(value);
        }};
    }
    changed!(compatibility_behavior, "exact-route-only");
    changed!(compatibility_boundary, "explicit-only");
    changed!(replacement_state, "candidate-required");
    changed!(active_reader_writer_state, "none-verified");
    changed!(observed_authority_state, "compatibility-route-retained");
    changed!(equivalence_proof, "verified");
    changed!(physical_cleanup_state, "preserve");
    for transition in transitions {
        let mut fixture = Fixture::baseline();
        fixture.registry[0] = RegistryRouteEvidence {
            transition,
            ..baseline
        };
        assert_eq!(fixture.verify(), Err(VerificationError::RegistryMismatch));
    }
}

#[test]
fn rejected_compatibility_route_retained_interpretation_stays_rejected() {
    let baseline = Fixture::baseline().registry[0];
    let mut old = baseline.transition;
    old.compatibility_behavior = "exact-route-only";
    old.compatibility_boundary = "explicit-only";
    old.replacement_state = "candidate-required";
    old.observed_authority_state = "compatibility-route-retained";
    old.physical_cleanup_state = "preserve";
    let mut fixture = Fixture::baseline();
    fixture.registry[0] = RegistryRouteEvidence {
        transition: old,
        ..baseline
    };
    assert_eq!(fixture.verify(), Err(VerificationError::RegistryMismatch));
}

#[test]
fn raw_premerge_source_target_registry_and_digest_conflicts_fail_closed() {
    let mut cases = Vec::new();

    let mut source_id = Fixture::baseline();
    source_id.sources[1].stable_id = source_id.sources[0].stable_id;
    cases.push(source_id);
    let mut source_path = Fixture::baseline();
    source_path.sources[1].path = source_path.sources[0].path;
    cases.push(source_path);

    let mut target_id = Fixture::baseline();
    target_id.targets[1].stable_id = target_id.targets[0].stable_id;
    cases.push(target_id);
    let mut target_path = Fixture::baseline();
    target_path.targets[1].path = target_path.targets[0].path;
    cases.push(target_path);

    let mut cross_id = Fixture::baseline();
    cross_id.sources[0].stable_id = cross_id.targets[0].stable_id;
    cases.push(cross_id);
    let mut cross_path = Fixture::baseline();
    cross_path.sources[0].path = cross_path.targets[0].path;
    cases.push(cross_path);

    let mut route_id = Fixture::baseline();
    route_id.registry[1].route_id = route_id.registry[0].route_id;
    cases.push(route_id);
    let mut matcher_id = Fixture::baseline();
    matcher_id.registry[1].matcher.stable_id = matcher_id.registry[0].matcher.stable_id;
    cases.push(matcher_id);
    let mut matcher_path = Fixture::baseline();
    matcher_path.registry[1].matcher.relative_path = matcher_path.registry[0].matcher.relative_path;
    cases.push(matcher_path);

    let mut digest_path = Fixture::baseline();
    digest_path.digests[1].path = digest_path.digests[0].path;
    cases.push(digest_path);

    for fixture in cases {
        assert_eq!(fixture.verify(), Err(VerificationError::Conflict));
    }
}
