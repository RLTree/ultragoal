use super::*;

#[test]
pub(crate) fn production_authority_fixture_catalog_is_exact_and_claimless() {
    let value: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../fixtures/routine-production-authority/cases.json"
    ))
    .unwrap();
    assert_eq!(
        value["schema_version"],
        "RoutineProductionAuthorityCases-v1"
    );
    assert_eq!(value["claim_effect"], "none");
    assert_eq!(value["supported_host"], "target_vendor=apple");
    let actual = value["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case.as_str().unwrap())
        .collect::<Vec<_>>();
    let expected = vec![
        "fresh-execution",
        "complete-no-op",
        "failed",
        "cancelled",
        "exact-reuse",
        "malformed-reuse-zero-transition",
        "forged-reuse-preopen-zero-transition",
        "foreign-record-reuse-zero-transition",
        "repeated-forgery-no-exhaustion",
        "reuse-preauthorization-generation-race",
        "forged-valid-reuse-concurrency-both-orders",
        "unauthenticated-reuse-complete-preservation",
        "consumed-grant-max-minus-one-max-max-plus-one",
        "protocol-effect-max-minus-one-max-max-plus-one",
        "capacity-concurrent-admission",
        "reserved-crash-recovery",
        "started-crash-recovery",
        "preterminal-crash-recovery",
        "expired-recovery-refusal",
        "cross-process-protocol-race",
        "stale-binding-refusal",
        "self-consistent-substitution",
        "owner-only-root",
        "unknown-entry",
        "key-hardlink",
        "state-symlink",
        "state-special-file",
        "lock-truncate",
        "root-replacement",
        "state-truncate",
        "state-unknown-field",
        "state-duplicate-field",
        "state-reorder",
        "state-rollback",
        "state-mutate-restore",
        "secret-path-output-redaction",
        "forged-test-grant-boundary",
    ];
    assert_eq!(actual, expected);
}

#[test]
pub(crate) fn production_fresh_execution_replay_refusal_and_exact_reuse_are_durable() {
    let fixture = fixture("production-fresh-reuse", true);
    let authority = AuthorityRoot::new("production-fresh-reuse");
    let first = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap();
    let reusable_artifacts = first.reuse_artifacts().to_vec();
    assert_eq!(first.status(), RoutineMediatorStatus::CompleteExecution);
    assert_eq!(first.nodes().len(), 1);
    assert_eq!(
        first.nodes()[0].disposition(),
        RoutineNodeDisposition::Executed
    );
    assert_eq!(
        fs::read(
            fixture
                .repo
                .root()
                .join("target/routine/compile/result.txt")
        )
        .unwrap(),
        b"compile"
    );
    assert_eq!(
        authority.tree().keys().cloned().collect::<Vec<_>>(),
        vec![
            "routine-authority.key",
            "routine-authority.lock",
            "routine-authority.state"
        ]
    );
    let before_target = fixture.repo.tree();
    let before_authority = authority.tree();
    let replay = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap_err();
    assert_eq!(
        replay.cause(),
        "routine-production-semantic-effect-replayed"
    );
    assert_eq!(fixture.repo.tree(), before_target);
    assert_eq!(authority.tree(), before_authority);

    let before_reuse = fixture.repo.tree();
    let reused = mediate(
        &authority,
        &fixture,
        prepared(&fixture),
        reusable_artifacts.clone(),
    )
    .unwrap();
    assert_eq!(reused.status(), RoutineMediatorStatus::CompleteExecution);
    assert_eq!(
        reused.nodes()[0].disposition(),
        RoutineNodeDisposition::Reused
    );
    assert_eq!(fixture.repo.tree(), before_reuse);

    drop(reused);
    drop(first);
    let reopened = ProductionRoutineIssuer::open(authority.path()).unwrap();
    assert!(
        reopened
            .pending_recovery(&fixture.context, &fixture.plan, &prepared(&fixture))
            .unwrap()
            .is_none()
    );

    let repeated = mediate(&authority, &fixture, prepared(&fixture), reusable_artifacts).unwrap();
    assert_eq!(repeated.status(), RoutineMediatorStatus::CompleteExecution);
    assert_eq!(
        repeated.nodes()[0].disposition(),
        RoutineNodeDisposition::Reused
    );
}

#[test]
pub(crate) fn malformed_reuse_refuses_before_any_authority_transition() {
    let fixture = fixture("production-malformed-reuse", true);
    let authority = AuthorityRoot::new("production-malformed-reuse");
    let before_target = fixture.repo.tree();
    let refused = mediate(
        &authority,
        &fixture,
        prepared(&fixture),
        vec![b"not-a-reuse-artifact".to_vec()],
    )
    .unwrap_err();
    assert_eq!(refused.cause(), "mediator-production-reuse-input-malformed");
    assert_eq!(authority.tree(), BTreeMap::new());
    assert_eq!(fixture.repo.tree(), before_target);

    let source = AuthorityRoot::new("production-forged-source");
    let valid = mediate(&source, &fixture, prepared(&fixture), Vec::new())
        .unwrap()
        .reuse_artifacts()
        .to_vec();
    let before_forged_tree = fixture.repo.tree();
    let before_forged_status = fixture.repo.status();
    let forged = mediate(
        &authority,
        &fixture,
        prepared(&fixture),
        vec![corrupt_reuse_witness(&valid[0])],
    )
    .unwrap_err();
    assert_eq!(
        forged.cause(),
        "mediator-production-reuse-not-authenticated"
    );
    assert_eq!(authority.tree(), BTreeMap::new());
    assert_eq!(fixture.repo.tree(), before_forged_tree);
    assert_eq!(fixture.repo.status(), before_forged_status);
}
