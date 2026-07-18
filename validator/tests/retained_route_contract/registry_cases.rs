use super::super::{
    EntryEvidence, MatcherEvidence, RegistryRouteEvidence, TransitionEvidence, VerificationError,
    routes,
};
use super::route_transition_fixture::Fixture;

#[test]
fn matcher_must_bind_stable_id_kind_and_path_exactly() {
    let spec = routes().next().unwrap();
    let baseline = Fixture::baseline().registry[0];
    let bad_matchers = [
        MatcherEvidence {
            stable_id: None,
            ..baseline.matcher
        },
        MatcherEvidence {
            stable_id: Some("LEGACY-COMMAND:wrong"),
            ..baseline.matcher
        },
        MatcherEvidence {
            kind: None,
            ..baseline.matcher
        },
        MatcherEvidence {
            kind: Some("legacy-lane-authority"),
            ..baseline.matcher
        },
        MatcherEvidence {
            relative_path: None,
            ..baseline.matcher
        },
        MatcherEvidence {
            relative_path: Some("validator/src/argument_parser"),
            ..baseline.matcher
        },
    ];
    for matcher in bad_matchers {
        let mut fixture = Fixture::baseline();
        fixture.registry[0] = RegistryRouteEvidence {
            matcher,
            ..baseline
        };
        assert_eq!(fixture.verify(), Err(VerificationError::RegistryMismatch));
    }
    assert_eq!(baseline.matcher.stable_id, Some(spec.stable_id));
    assert_eq!(baseline.matcher.kind, Some(spec.kind));
    assert_eq!(baseline.matcher.relative_path, Some(spec.path));
}

#[test]
fn route_target_disposition_and_proof_tamper_fail_closed() {
    const WRONG_REFS: [&str; 3] = ["wrong", "wrong-target", "wrong-result"];
    let baseline = Fixture::baseline().registry[0];
    let bad_routes = [
        RegistryRouteEvidence {
            canonical_target: "PS-WRONG",
            ..baseline
        },
        RegistryRouteEvidence {
            intended_disposition: "retired",
            ..baseline
        },
        RegistryRouteEvidence {
            transition: TransitionEvidence {
                proof_refs: &WRONG_REFS,
                ..baseline.transition
            },
            ..baseline
        },
    ];
    for route in bad_routes {
        let mut fixture = Fixture::baseline();
        fixture.registry[0] = route;
        assert_eq!(fixture.verify(), Err(VerificationError::RegistryMismatch));
    }

    let mut unknown = Fixture::baseline();
    unknown.registry[0] = RegistryRouteEvidence {
        route_id: "unknown-route",
        ..baseline
    };
    assert_eq!(unknown.verify(), Err(VerificationError::UnknownRoute));

    let mut missing = Fixture::baseline();
    missing.registry.pop();
    assert_eq!(missing.verify(), Err(VerificationError::RouteSetMismatch));
}

#[test]
fn target_definition_identity_and_state_tamper_fail_closed() {
    let baseline = Fixture::baseline().targets[0];
    let wrong = [
        EntryEvidence {
            stable_id: "PS-WRONG",
            ..baseline
        },
        EntryEvidence {
            kind: "wrong",
            ..baseline
        },
        EntryEvidence {
            path: "wrong",
            ..baseline
        },
        EntryEvidence {
            sha256: "00",
            ..baseline
        },
        EntryEvidence {
            authority_state: "legacy",
            ..baseline
        },
        EntryEvidence {
            active_status: "context-only",
            ..baseline
        },
    ];
    for target in wrong {
        let mut fixture = Fixture::baseline();
        fixture.targets[0] = target;
        let expected = if target.stable_id == "PS-WRONG" {
            VerificationError::TargetMissing
        } else {
            VerificationError::TargetMismatch
        };
        assert_eq!(fixture.verify(), Err(expected));
    }

    let mut active = Fixture::baseline();
    active.targets[0].active_status = "active";
    assert_eq!(
        active.verify(),
        Err(VerificationError::ActiveCanonicalImplementation)
    );
}
