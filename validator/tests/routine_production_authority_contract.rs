#![allow(dead_code, unused_imports)]

mod context {
    pub use ultragoal::context::*;
}

#[path = "../src/cli/capture/mod.rs"]
mod capture;
#[path = "../src/routine_work/mod.rs"]
mod routine_work;
#[path = "routine_work_contract/support.rs"]
mod support;

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use context::{BuildRequest, LiveContext};
use routine_work::{
    CheckClass, DirtySnapshot, ImpactGraph, LocalDirtyTree, PathMatcher, PlanMode, PlanRequest,
    PreparedRoutineExecution, ProductionRoutineIssuer, RoutineAdapterSpec, RoutineCancellation,
    RoutineInvocationSpec, RoutineMediationResult, RoutineMediatorStatus, RoutineNodeDisposition,
    RoutinePlan, RoutineReuseInput, bind_routine_invocation,
    mediate_prepared_routine_execution_production, plan_routine, prepare_routine_execution,
    set_test_mediator_finish_failure,
};
use support::{TempRepo, node, path, route, sha};

static NEXT_AUTHORITY: AtomicU64 = AtomicU64::new(1);

struct Fixture {
    repo: TempRepo,
    context: LiveContext,
    graph: ImpactGraph,
    snapshot: DirtySnapshot,
    plan: RoutinePlan,
}

struct AuthorityRoot {
    parent: PathBuf,
    path: PathBuf,
}

impl AuthorityRoot {
    fn new(label: &str) -> Self {
        let sequence = NEXT_AUTHORITY.fetch_add(1, Ordering::Relaxed);
        let parent = std::env::temp_dir().join(format!(
            "hul-routine-production-{label}-{}-{sequence}",
            std::process::id()
        ));
        let path = parent.join("authority");
        fs::create_dir_all(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self { parent, path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn tree(&self) -> BTreeMap<String, String> {
        tree(&self.path)
    }
}

impl Drop for AuthorityRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.parent);
    }
}

fn fixture(label: &str, dirty: bool) -> Fixture {
    let repo = TempRepo::new(label);
    repo.write(".git/info/exclude", b"target/\n");
    fs::create_dir_all(repo.root().join("target/routine/compile")).unwrap();
    if dirty {
        repo.write("src/lib.rs", b"pub fn value() -> u8 { 89 }\n");
    }
    fixture_from_repo(repo, "routine-production")
}

fn fixture_from_repo(repo: TempRepo, profile: &str) -> Fixture {
    let context = LiveContext::build(
        BuildRequest::new(repo.root())
            .bind_non_secret_configuration("profile", profile)
            .probe_tool("sandbox-exec")
            .probe_tool("dash"),
    )
    .unwrap();
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = graph();
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).unwrap();
    Fixture {
        repo,
        context,
        graph,
        snapshot,
        plan,
    }
}

fn graph() -> ImpactGraph {
    ImpactGraph::new(
        vec![node("compile", &[], CheckClass::Routine, "dash", None)],
        vec![route(
            "route-src",
            PathMatcher::Prefix(path("src")),
            &["compile"],
            false,
        )],
        Vec::new(),
    )
    .unwrap()
}

fn prepared(fixture: &Fixture) -> PreparedRoutineExecution {
    let invocations = fixture
        .plan
        .checks()
        .iter()
        .map(|check| {
            bind_routine_invocation(
                &fixture.context,
                &fixture.plan,
                check.node_id(),
                vec!["-c".to_owned(), command_script(check.node_id())],
                10_000,
                1024 * 1024,
                vec![path(&format!("target/routine/{}", check.node_id()))],
            )
            .unwrap()
        })
        .collect();
    prepare_routine_execution(
        &fixture.context,
        &fixture.graph,
        &fixture.snapshot,
        &fixture.plan,
        RoutineAdapterSpec::new("routine", invocations),
    )
    .unwrap()
}

fn prepared_failure(fixture: &Fixture) -> PreparedRoutineExecution {
    let invocations = fixture
        .plan
        .checks()
        .iter()
        .map(|check| {
            bind_routine_invocation(
                &fixture.context,
                &fixture.plan,
                check.node_id(),
                vec!["-c".to_owned(), "exit 7".to_owned()],
                10_000,
                1024 * 1024,
                vec![path(&format!("target/routine/{}", check.node_id()))],
            )
            .unwrap()
        })
        .collect();
    prepare_routine_execution(
        &fixture.context,
        &fixture.graph,
        &fixture.snapshot,
        &fixture.plan,
        RoutineAdapterSpec::new("routine", invocations),
    )
    .unwrap()
}

fn command_script(node_id: &str) -> String {
    format!(
        "printf '%s' '{node_id}' > 'target/routine/{node_id}/result.txt'; printf '{{\"schema_version\":\"RoutineCommandReport-v1\",\"request_id\":\"%s\",\"protocol_id\":\"%s\",\"intent_id\":\"%s\",\"node_id\":\"%s\",\"outcome\":\"passed\",\"behavior_observed\":true}}' \"$HUL_ROUTINE_REQUEST_ID\" \"$HUL_ROUTINE_PROTOCOL_ID\" \"$HUL_ROUTINE_INTENT_ID\" \"$HUL_ROUTINE_NODE_ID\""
    )
}

fn mediate(
    authority: &AuthorityRoot,
    fixture: &Fixture,
    prepared: PreparedRoutineExecution,
    reuse: Vec<Vec<u8>>,
) -> Result<RoutineMediationResult, routine_work::RoutineError> {
    mediate_prepared_routine_execution_production(
        authority.path(),
        &fixture.context,
        &fixture.plan,
        prepared,
        None,
        RoutineCancellation::new(),
        RoutineReuseInput::new(reuse),
    )
}

fn authority_state(authority: &AuthorityRoot) -> Vec<u8> {
    fs::read(authority.path().join("routine-authority.state")).unwrap()
}

fn authority_cardinalities(authority: &AuthorityRoot) -> (usize, usize, usize) {
    let state: serde_json::Value = serde_json::from_slice(&authority_state(authority)).unwrap();
    let payload = &state["payload"];
    (
        payload["protocols"].as_object().unwrap().len(),
        payload["effects"].as_object().unwrap().len(),
        payload["consumed_grants"].as_array().unwrap().len(),
    )
}

fn foreign_reuse_for_same_request(fixture: &Fixture, label: &str) -> Vec<Vec<u8>> {
    let authority = AuthorityRoot::new(label);
    mediate(&authority, fixture, prepared(fixture), Vec::new())
        .unwrap()
        .reuse_artifacts()
        .to_vec()
}

fn corrupt_reuse_witness(bytes: &[u8]) -> Vec<u8> {
    let mut forged = String::from_utf8(bytes.to_vec()).unwrap();
    let wire: serde_json::Value = serde_json::from_str(&forged).unwrap();
    let witness = wire["mediator_witness_sha256"].as_str().unwrap();
    forged = forged.replace(
        witness,
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    );
    forged.into_bytes()
}

#[test]
fn production_authority_fixture_catalog_is_exact_and_claimless() {
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
fn production_fresh_execution_replay_refusal_and_exact_reuse_are_durable() {
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
fn malformed_reuse_refuses_before_any_authority_transition() {
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

#[test]
fn invalid_reuse_never_regresses_complete_or_blocks_later_exact_reuse() {
    let fixture = fixture("production-invalid-reuse-preserves-complete", true);
    let authority = AuthorityRoot::new("production-invalid-reuse-preserves-complete");
    let first = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap();
    let valid_reuse = first.reuse_artifacts().to_vec();

    let before_malformed = authority.tree();
    let malformed = mediate(
        &authority,
        &fixture,
        prepared(&fixture),
        vec![b"not-a-reuse-artifact".to_vec()],
    )
    .unwrap_err();
    assert_eq!(
        malformed.cause(),
        "mediator-production-reuse-input-malformed"
    );
    assert_eq!(authority.tree(), before_malformed);

    let exact_after_malformed = mediate(
        &authority,
        &fixture,
        prepared(&fixture),
        valid_reuse.clone(),
    )
    .unwrap();
    assert_eq!(
        exact_after_malformed.nodes()[0].disposition(),
        RoutineNodeDisposition::Reused
    );

    let forged = corrupt_reuse_witness(&valid_reuse[0]);
    let before_forgery_state = authority_state(&authority);
    let before_forgery_cardinalities = authority_cardinalities(&authority);
    let before_forgery_tree = fixture.repo.tree();
    let before_forgery_status = fixture.repo.status();
    let refused = mediate(&authority, &fixture, prepared(&fixture), vec![forged]).unwrap_err();
    assert_eq!(
        refused.cause(),
        "mediator-production-reuse-not-authenticated"
    );
    assert_eq!(authority_state(&authority), before_forgery_state);
    assert_eq!(
        authority_cardinalities(&authority),
        before_forgery_cardinalities
    );
    assert_eq!(fixture.repo.tree(), before_forgery_tree);
    assert_eq!(fixture.repo.status(), before_forgery_status);

    // A foreign artifact is fully canonical, internally self-consistent, and
    // process-authenticated, but it belongs to a different durable Complete
    // record. Repeating it must not consume grants or create an exhaustion
    // path in this authority store.
    let foreign_reuse = foreign_reuse_for_same_request(&fixture, "production-foreign-complete");
    let local_wire: serde_json::Value = serde_json::from_slice(&valid_reuse[0]).unwrap();
    let foreign_wire: serde_json::Value = serde_json::from_slice(&foreign_reuse[0]).unwrap();
    assert_eq!(local_wire["protocol_id"], foreign_wire["protocol_id"]);
    assert_eq!(local_wire["intent_id"], foreign_wire["intent_id"]);
    assert_ne!(valid_reuse[0], foreign_reuse[0]);
    let before_substitution_state = authority_state(&authority);
    let before_substitution_cardinalities = authority_cardinalities(&authority);
    let before_substitution_tree = fixture.repo.tree();
    let before_substitution_status = fixture.repo.status();
    for _ in 0..64 {
        let error = mediate(
            &authority,
            &fixture,
            prepared(&fixture),
            foreign_reuse.clone(),
        )
        .unwrap_err();
        assert_eq!(error.cause(), "mediator-production-reuse-not-authenticated");
    }
    assert_eq!(authority_state(&authority), before_substitution_state);
    assert_eq!(
        authority_cardinalities(&authority),
        before_substitution_cardinalities
    );
    assert_eq!(fixture.repo.tree(), before_substitution_tree);
    assert_eq!(fixture.repo.status(), before_substitution_status);

    ProductionRoutineIssuer::open(authority.path()).unwrap();

    let exact_after_forgery =
        mediate(&authority, &fixture, prepared(&fixture), valid_reuse).unwrap();
    assert_eq!(
        exact_after_forgery.nodes()[0].disposition(),
        RoutineNodeDisposition::Reused
    );
}

#[test]
fn reuse_preauthorization_generation_race_fails_before_reservation_mutation() {
    let fixture = fixture("production-reuse-preauthorization-race", true);
    let authority = AuthorityRoot::new("production-reuse-preauthorization-race");
    let first = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap();
    let reuse = first.reuse_artifacts().to_vec();
    assert_eq!(authority_cardinalities(&authority), (1, 1, 1));

    let raced_state = Arc::new(Mutex::new(None));
    let raced_state_from_hook = Arc::clone(&raced_state);
    let authority_root = authority.path().to_path_buf();
    ProductionRoutineIssuer::test_set_reuse_preauthorization_hook(move || {
        let contender = ProductionRoutineIssuer::open(&authority_root).unwrap();
        contender.test_seed_capacity(1, 1).unwrap();
        *raced_state_from_hook.lock().unwrap() =
            Some(fs::read(authority_root.join("routine-authority.state")).unwrap());
    });
    let before_target = fixture.repo.tree();
    let before_status = fixture.repo.status();
    let refused = mediate(&authority, &fixture, prepared(&fixture), reuse.clone()).unwrap_err();
    assert_eq!(
        refused.cause(),
        "routine-production-reuse-preauthorization-stale"
    );
    let after_contender = raced_state.lock().unwrap().take().unwrap();
    assert_eq!(authority_state(&authority), after_contender);
    assert_eq!(authority_cardinalities(&authority), (1, 1, 1));
    assert_eq!(fixture.repo.tree(), before_target);
    assert_eq!(fixture.repo.status(), before_status);

    let reopened = mediate(&authority, &fixture, prepared(&fixture), reuse).unwrap();
    assert_eq!(
        reopened.nodes()[0].disposition(),
        RoutineNodeDisposition::Reused
    );
}

#[test]
fn consumed_grant_max_minus_one_max_and_max_plus_one_are_fail_closed() {
    let (record_limit, consumed_limit) = ProductionRoutineIssuer::test_capacity_limits();
    assert_eq!((record_limit, consumed_limit), (4_096, 16_384));
    let fixture = fixture("production-consumed-capacity", true);
    let authority = AuthorityRoot::new("production-consumed-capacity");
    let first = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap();
    let reuse = first.reuse_artifacts().to_vec();
    let foreign_reuse =
        foreign_reuse_for_same_request(&fixture, "production-consumed-capacity-foreign");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    issuer.test_seed_capacity(1, consumed_limit - 1).unwrap();
    assert_eq!(
        authority_cardinalities(&authority),
        (1, 1, consumed_limit - 1)
    );

    let at_max = mediate(&authority, &fixture, prepared(&fixture), reuse.clone()).unwrap();
    assert_eq!(
        at_max.nodes()[0].disposition(),
        RoutineNodeDisposition::Reused
    );
    assert_eq!(authority_cardinalities(&authority), (1, 1, consumed_limit));
    let max_state = authority_state(&authority);

    let forged_at_max =
        mediate(&authority, &fixture, prepared(&fixture), foreign_reuse).unwrap_err();
    assert_eq!(
        forged_at_max.cause(),
        "mediator-production-reuse-not-authenticated"
    );
    assert_eq!(authority_state(&authority), max_state);

    let over_limit = mediate(&authority, &fixture, prepared(&fixture), reuse).unwrap_err();
    assert_eq!(
        over_limit.cause(),
        "routine-production-authority-capacity-exhausted"
    );
    assert_eq!(authority_state(&authority), max_state);

    let reopened = ProductionRoutineIssuer::open(authority.path()).unwrap();
    assert!(
        reopened
            .pending_recovery(&fixture.context, &fixture.plan, &prepared(&fixture))
            .unwrap()
            .is_none()
    );
    let replay = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap_err();
    assert_eq!(
        replay.cause(),
        "routine-production-semantic-effect-replayed"
    );
    assert_eq!(authority_state(&authority), max_state);
}

#[test]
fn protocol_effect_max_minus_one_max_and_max_plus_one_are_fail_closed() {
    let (record_limit, consumed_limit) = ProductionRoutineIssuer::test_capacity_limits();
    let fixture_at_max = fixture("production-record-capacity-max", true);
    let fixture_over_limit = fixture("production-record-capacity-over", true);
    let authority = AuthorityRoot::new("production-record-capacity");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    issuer
        .test_seed_capacity(record_limit - 1, record_limit - 1)
        .unwrap();
    assert_eq!(
        authority_cardinalities(&authority),
        (record_limit - 1, record_limit - 1, record_limit - 1)
    );

    let request_at_max = prepared(&fixture_at_max);
    issuer
        .test_reserve_and_abandon(
            &fixture_at_max.context,
            &fixture_at_max.plan,
            &request_at_max,
            false,
        )
        .unwrap();
    assert_eq!(
        authority_cardinalities(&authority),
        (record_limit, record_limit, record_limit)
    );

    let reopened = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let recovery_request = prepared(&fixture_at_max);
    let recovery = reopened
        .pending_recovery(
            &fixture_at_max.context,
            &fixture_at_max.plan,
            &recovery_request,
        )
        .unwrap()
        .expect("the exact maximum record remains recoverable after reopen");
    let recovered = reopened
        .mediate(
            &fixture_at_max.context,
            &fixture_at_max.plan,
            recovery_request,
            Some(recovery),
            RoutineCancellation::new(),
            RoutineReuseInput::new(Vec::new()),
        )
        .unwrap();
    assert_eq!(recovered.status(), RoutineMediatorStatus::CompleteExecution);
    assert_eq!(
        authority_cardinalities(&authority),
        (record_limit, record_limit, record_limit + 1)
    );
    assert!(record_limit + 1 < consumed_limit);
    let max_state = authority_state(&authority);

    let over_limit_request = prepared(&fixture_over_limit);
    let refused = reopened
        .test_reserve_and_abandon(
            &fixture_over_limit.context,
            &fixture_over_limit.plan,
            &over_limit_request,
            false,
        )
        .unwrap_err();
    assert_eq!(
        refused.cause(),
        "routine-production-authority-capacity-exhausted"
    );
    assert_eq!(authority_state(&authority), max_state);
    assert_eq!(
        authority_cardinalities(&authority),
        (record_limit, record_limit, record_limit + 1)
    );
    ProductionRoutineIssuer::open(authority.path()).unwrap();

    let replay = mediate(
        &authority,
        &fixture_at_max,
        prepared(&fixture_at_max),
        Vec::new(),
    )
    .unwrap_err();
    assert_eq!(
        replay.cause(),
        "routine-production-semantic-effect-replayed"
    );
    assert_eq!(authority_state(&authority), max_state);
}

#[test]
fn no_op_bypasses_authority_initialization_and_all_writes() {
    let fixture = fixture("production-noop", false);
    assert_eq!(fixture.plan.affected_set().mode(), PlanMode::NoOp);
    let authority = AuthorityRoot::new("production-noop");
    fs::remove_dir(authority.path()).unwrap();
    let before = fixture.repo.tree();
    let prepared = prepare_routine_execution(
        &fixture.context,
        &fixture.graph,
        &fixture.snapshot,
        &fixture.plan,
        RoutineAdapterSpec::new("routine", Vec::new()),
    )
    .unwrap();
    let result = mediate_prepared_routine_execution_production(
        authority.path(),
        &fixture.context,
        &fixture.plan,
        prepared,
        None,
        RoutineCancellation::new(),
        RoutineReuseInput::new(Vec::new()),
    )
    .unwrap();
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteNoOp);
    assert!(!authority.path().exists());
    assert_eq!(fixture.repo.tree(), before);
}

#[test]
fn reserved_and_started_crashes_require_exact_bounded_recovery() {
    for started in [false, true] {
        let fixture = fixture(
            if started {
                "production-crash-started"
            } else {
                "production-crash-reserved"
            },
            true,
        );
        let authority = AuthorityRoot::new(if started {
            "production-crash-started"
        } else {
            "production-crash-reserved"
        });
        let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
        let request = prepared(&fixture);
        issuer
            .test_reserve_and_abandon(&fixture.context, &fixture.plan, &request, started)
            .unwrap();
        let before_target = fixture.repo.tree();
        let before_authority = authority.tree();
        let replay = issuer
            .mediate(
                &fixture.context,
                &fixture.plan,
                request,
                None,
                RoutineCancellation::new(),
                RoutineReuseInput::new(Vec::new()),
            )
            .unwrap_err();
        assert_eq!(
            replay.cause(),
            "routine-production-semantic-effect-replayed"
        );
        assert_eq!(fixture.repo.tree(), before_target);
        assert_eq!(authority.tree(), before_authority);

        let recovery_request = prepared(&fixture);
        let recovery = issuer
            .pending_recovery(&fixture.context, &fixture.plan, &recovery_request)
            .unwrap()
            .expect("pending recovery authority");
        let recovered = issuer
            .mediate(
                &fixture.context,
                &fixture.plan,
                recovery_request,
                Some(recovery),
                RoutineCancellation::new(),
                RoutineReuseInput::new(Vec::new()),
            )
            .unwrap();
        assert_eq!(recovered.status(), RoutineMediatorStatus::CompleteExecution);
        assert!(
            issuer
                .pending_recovery(&fixture.context, &fixture.plan, &prepared(&fixture))
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn expired_recovery_authority_is_rejected_from_authenticated_state() {
    let fixture = fixture("production-expired-recovery", true);
    let authority = AuthorityRoot::new("production-expired-recovery");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let request = prepared(&fixture);
    issuer
        .test_reserve_and_abandon(&fixture.context, &fixture.plan, &request, true)
        .unwrap();
    issuer
        .test_expire_pending(&fixture.context, &fixture.plan, &request)
        .unwrap();
    let before_target = fixture.repo.tree();
    let before_authority = authority.tree();
    let refused = issuer
        .pending_recovery(&fixture.context, &fixture.plan, &request)
        .unwrap_err();
    assert_eq!(refused.cause(), "routine-production-recovery-expired");
    assert_eq!(fixture.repo.tree(), before_target);
    assert_eq!(authority.tree(), before_authority);
}

#[test]
fn preterminal_reconciliation_failure_remains_pending_across_reopen() {
    let fixture = fixture("production-preterminal-crash", true);
    let authority = AuthorityRoot::new("production-preterminal-crash");
    set_test_mediator_finish_failure();
    let failed = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap_err();
    assert_eq!(failed.cause(), "adapter-mediation-transition-incomplete");

    let reopened = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let recovery_request = prepared(&fixture);
    let recovery = reopened
        .pending_recovery(&fixture.context, &fixture.plan, &recovery_request)
        .unwrap()
        .expect("started record remains pending");
    let result = reopened
        .mediate(
            &fixture.context,
            &fixture.plan,
            recovery_request,
            Some(recovery),
            RoutineCancellation::new(),
            RoutineReuseInput::new(Vec::new()),
        )
        .unwrap();
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteExecution);
}

#[test]
fn failure_and_cancellation_settle_terminally_without_recovery() {
    let failed_fixture = fixture("production-failed", true);
    let failed_authority = AuthorityRoot::new("production-failed");
    let failed = mediate(
        &failed_authority,
        &failed_fixture,
        prepared_failure(&failed_fixture),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(failed.status(), RoutineMediatorStatus::IncompleteExecution);
    let failed_issuer = ProductionRoutineIssuer::open(failed_authority.path()).unwrap();
    assert!(
        failed_issuer
            .pending_recovery(
                &failed_fixture.context,
                &failed_fixture.plan,
                &prepared_failure(&failed_fixture),
            )
            .unwrap()
            .is_none()
    );

    let cancelled_fixture = fixture("production-cancelled", true);
    let cancelled_authority = AuthorityRoot::new("production-cancelled");
    let cancellation = RoutineCancellation::new();
    cancellation.cancel();
    let cancelled = mediate_prepared_routine_execution_production(
        cancelled_authority.path(),
        &cancelled_fixture.context,
        &cancelled_fixture.plan,
        prepared(&cancelled_fixture),
        None,
        cancellation,
        RoutineReuseInput::new(Vec::new()),
    )
    .unwrap();
    assert_eq!(cancelled.status(), RoutineMediatorStatus::Cancelled);
    let cancelled_issuer = ProductionRoutineIssuer::open(cancelled_authority.path()).unwrap();
    assert!(
        cancelled_issuer
            .pending_recovery(
                &cancelled_fixture.context,
                &cancelled_fixture.plan,
                &prepared(&cancelled_fixture),
            )
            .unwrap()
            .is_none()
    );
}

#[test]
fn stale_request_and_self_consistent_substitution_refuse_without_hidden_writes() {
    let stale_fixture = fixture("production-stale", true);
    let authority = AuthorityRoot::new("production-stale");
    let stale = prepared(&stale_fixture);
    stale_fixture
        .repo
        .write("src/late.rs", b"pub fn late() {}\n");
    let before_target = stale_fixture.repo.tree();
    let before_authority = authority.tree();
    let refused = mediate(&authority, &stale_fixture, stale, Vec::new()).unwrap_err();
    assert!(refused.cause().contains("stale") || refused.cause().contains("mutated"));
    assert_eq!(stale_fixture.repo.tree(), before_target);
    assert_eq!(authority.tree(), before_authority);

    let original = fixture("production-substitution-original", true);
    let substitute = fixture("production-substitution-coherent", true);
    let authority = AuthorityRoot::new("production-substitution");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let original_request = prepared(&original);
    issuer
        .test_reserve_and_abandon(&original.context, &original.plan, &original_request, true)
        .unwrap();
    let recovery = issuer
        .pending_recovery(&original.context, &original.plan, &original_request)
        .unwrap()
        .unwrap();
    let before_original = original.repo.tree();
    let before_substitute = substitute.repo.tree();
    let before_authority = authority.tree();
    let refused = issuer
        .mediate(
            &substitute.context,
            &substitute.plan,
            prepared(&substitute),
            Some(recovery),
            RoutineCancellation::new(),
            RoutineReuseInput::new(Vec::new()),
        )
        .unwrap_err();
    assert_eq!(
        refused.cause(),
        "routine-production-recovery-authority-stale"
    );
    assert_eq!(original.repo.tree(), before_original);
    assert_eq!(substitute.repo.tree(), before_substitute);
    assert_eq!(authority.tree(), before_authority);
}

#[test]
fn owner_only_store_rejects_unknown_hardlink_symlink_special_and_root_replacement() {
    mutation_case("unknown", |root| {
        fs::write(root.join("unknown"), b"x").unwrap()
    });
    mutation_case("key-hardlink", |root| {
        fs::hard_link(root.join("routine-authority.key"), root.join("key-alias")).unwrap();
    });
    mutation_case("state-symlink", |root| {
        let state = root.join("routine-authority.state");
        let saved = root.join("outside-state");
        fs::rename(&state, &saved).unwrap();
        symlink(&saved, &state).unwrap();
    });
    mutation_case("state-fifo", |root| {
        let state = root.join("routine-authority.state");
        fs::remove_file(&state).unwrap();
        let name = std::ffi::CString::new(state.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    });
    mutation_case("lock-truncate", |root| {
        fs::write(root.join("routine-authority.lock"), b"").unwrap();
    });
    mutation_case("root-mode", |root| {
        fs::set_permissions(root, fs::Permissions::from_mode(0o755)).unwrap();
    });

    let fixture = fixture("production-root-replaced", true);
    let authority = AuthorityRoot::new("production-root-replaced");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let request = prepared(&fixture);
    issuer
        .test_reserve_and_abandon(&fixture.context, &fixture.plan, &request, false)
        .unwrap();
    let displaced = authority.parent.join("displaced");
    fs::rename(authority.path(), &displaced).unwrap();
    fs::create_dir(authority.path()).unwrap();
    fs::set_permissions(authority.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let error = issuer
        .pending_recovery(&fixture.context, &fixture.plan, &request)
        .unwrap_err();
    assert_eq!(error.cause(), "routine-production-authority-root-replaced");
}

fn mutation_case(label: &str, mutate: impl FnOnce(&Path)) {
    let fixture = fixture(&format!("production-{label}"), true);
    let authority = AuthorityRoot::new(&format!("production-{label}"));
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let request = prepared(&fixture);
    issuer
        .test_reserve_and_abandon(&fixture.context, &fixture.plan, &request, false)
        .unwrap();
    mutate(authority.path());
    let error = issuer
        .pending_recovery(&fixture.context, &fixture.plan, &request)
        .unwrap_err();
    assert!(
        error.cause().starts_with("routine-production-authority-"),
        "unexpected diagnostic: {error}"
    );
    assert!(
        !error
            .to_string()
            .contains(&authority.path().to_string_lossy().as_ref())
    );
}

#[test]
fn authenticated_state_rejects_truncate_unknown_duplicate_reorder_rollback_and_mutate_restore() {
    state_mutation_case("truncate", |bytes| bytes.truncate(bytes.len() / 2));
    state_mutation_case("unknown", |bytes| {
        let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        value["unknown"] = serde_json::json!(true);
        *bytes = serde_json::to_vec(&value).unwrap();
    });
    state_mutation_case("reorder", |bytes| bytes.reverse());
    state_mutation_case("duplicate", |bytes| {
        let text = String::from_utf8(bytes.clone()).unwrap();
        *bytes = text.replacen("{", "{\"payload\":null,", 1).into_bytes();
    });

    let rollback_fixture = fixture("production-rollback", true);
    let authority = AuthorityRoot::new("production-rollback");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let state_path = authority.path().join("routine-authority.state");
    let old = fs::read(&state_path).unwrap();
    let request = prepared(&rollback_fixture);
    issuer
        .test_reserve_and_abandon(
            &rollback_fixture.context,
            &rollback_fixture.plan,
            &request,
            false,
        )
        .unwrap();
    fs::write(&state_path, old).unwrap();
    let rollback = issuer
        .pending_recovery(&rollback_fixture.context, &rollback_fixture.plan, &request)
        .unwrap_err();
    assert_eq!(
        rollback.cause(),
        "routine-production-authority-rollback-detected"
    );

    let fixture = fixture("production-mutate-restore", true);
    let authority = AuthorityRoot::new("production-mutate-restore");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let request = prepared(&fixture);
    issuer
        .test_reserve_and_abandon(&fixture.context, &fixture.plan, &request, false)
        .unwrap();
    let state_path = authority.path().join("routine-authority.state");
    let original = fs::read(&state_path).unwrap();
    fs::write(&state_path, b"mutated").unwrap();
    fs::write(&state_path, original).unwrap();
    let restored = issuer
        .pending_recovery(&fixture.context, &fixture.plan, &request)
        .unwrap_err();
    assert_eq!(
        restored.cause(),
        "routine-production-authority-rollback-detected"
    );
}

fn state_mutation_case(label: &str, mutate: impl FnOnce(&mut Vec<u8>)) {
    let fixture = fixture(&format!("production-state-{label}"), true);
    let authority = AuthorityRoot::new(&format!("production-state-{label}"));
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let request = prepared(&fixture);
    issuer
        .test_reserve_and_abandon(&fixture.context, &fixture.plan, &request, false)
        .unwrap();
    let state = authority.path().join("routine-authority.state");
    let mut bytes = fs::read(&state).unwrap();
    mutate(&mut bytes);
    fs::write(&state, bytes).unwrap();
    let error = issuer
        .pending_recovery(&fixture.context, &fixture.plan, &request)
        .unwrap_err();
    assert!(
        error
            .cause()
            .starts_with("routine-production-authority-state-")
    );
}

#[test]
fn secrets_paths_and_raw_output_are_absent_from_authority_and_diagnostics() {
    let fixture = fixture("production-redaction", true);
    let authority = AuthorityRoot::new("production-redaction");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let request = prepared(&fixture);
    issuer
        .test_reserve_and_abandon(&fixture.context, &fixture.plan, &request, false)
        .unwrap();
    let key = fs::read(authority.path().join("routine-authority.key")).unwrap();
    let state = fs::read(authority.path().join("routine-authority.state")).unwrap();
    let state_text = String::from_utf8(state.clone()).unwrap();
    assert!(!state.windows(key.len()).any(|window| window == key));
    assert!(!state_text.contains(&fixture.repo.root().to_string_lossy().as_ref()));
    assert!(!state_text.contains("compile\n"));
    let error = issuer
        .mediate(
            &fixture.context,
            &fixture.plan,
            request,
            None,
            RoutineCancellation::new(),
            RoutineReuseInput::new(Vec::new()),
        )
        .unwrap_err();
    let diagnostic = error.to_string();
    assert!(!diagnostic.contains(&authority.path().to_string_lossy().as_ref()));
    assert!(!diagnostic.contains(&fixture.repo.root().to_string_lossy().as_ref()));
}

#[test]
fn production_boundary_has_one_sealed_issuer_and_no_test_grant_entrypoint() {
    let runtime = include_str!("../src/routine_work/runtime_adapter.rs");
    let production = include_str!("../src/routine_work/runtime_adapter/production.rs");
    let ledger = include_str!("../src/routine_work/runtime_adapter/production/ledger.rs");
    let model = include_str!("../src/routine_work/runtime_adapter/mediator/model.rs");
    assert_eq!(production.matches("issue_production_grant(").count(), 1);
    assert!(production.contains("pub(crate) struct ProductionRoutineIssuer"));
    assert!(production.contains("preflight_production_request"));
    assert!(!production.contains("RoutineRootGrant::test_issue"));
    assert!(model.contains("#[cfg(test)]\nimpl RoutineRootGrant"));
    assert!(runtime.contains("mod production;"));
    assert!(!production.contains("ClaimDecision"));
    assert!(!production.contains("public command"));
    assert!(ledger.contains("pub(super) struct ReusePreauthorization"));
    let marker = "pub(super) struct ReusePreauthorization";
    let offset = ledger.find(marker).unwrap();
    let attributes = &ledger[offset.saturating_sub(180)..offset];
    let authorization = ledger[offset + marker.len()..]
        .split("\n}\n")
        .next()
        .unwrap();
    assert!(!attributes.contains("derive(Clone"));
    assert!(!attributes.contains("Serialize"));
    assert!(!attributes.contains("Deserialize"));
    assert!(!authorization.contains("pub("));
    assert_eq!(ledger.matches("ReusePreauthorization {").count(), 2);
}

#[test]
#[ignore = "spawned explicitly by the cross-process race test"]
fn production_child_race_attempt() {
    if std::env::var_os("HUL_ROUTINE_PRODUCTION_CHILD").is_none() {
        return;
    }
    let repo_root = PathBuf::from(std::env::var_os("HUL_ROUTINE_REPO").unwrap());
    let authority_root = PathBuf::from(std::env::var_os("HUL_ROUTINE_AUTHORITY").unwrap());
    let outcome = PathBuf::from(std::env::var_os("HUL_ROUTINE_OUTCOME").unwrap());
    let context = LiveContext::build(
        BuildRequest::new(&repo_root)
            .bind_non_secret_configuration("profile", "routine-production-race")
            .probe_tool("sandbox-exec")
            .probe_tool("dash"),
    )
    .unwrap();
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = graph();
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).unwrap();
    let invocation = bind_routine_invocation(
        &context,
        &plan,
        "compile",
        vec!["-c".to_owned(), command_script("compile")],
        10_000,
        1024 * 1024,
        vec![path("target/routine/compile")],
    )
    .unwrap();
    let prepared = prepare_routine_execution(
        &context,
        &graph,
        &snapshot,
        &plan,
        RoutineAdapterSpec::new("routine", vec![invocation]),
    )
    .unwrap();
    let reuse = std::env::var_os("HUL_ROUTINE_REUSE")
        .map(|path| vec![fs::read(path).unwrap()])
        .unwrap_or_default();
    let result = mediate_prepared_routine_execution_production(
        &authority_root,
        &context,
        &plan,
        prepared,
        None,
        RoutineCancellation::new(),
        RoutineReuseInput::new(reuse),
    );
    let text = match result {
        Ok(_) => "complete".to_owned(),
        Err(error) => error.cause().to_owned(),
    };
    fs::write(outcome, text).unwrap();
}

#[test]
fn two_processes_racing_the_same_protocol_have_exactly_one_winner() {
    let repo = TempRepo::new("production-process-race");
    repo.write(".git/info/exclude", b"target/\n");
    fs::create_dir_all(repo.root().join("target/routine/compile")).unwrap();
    repo.write("src/lib.rs", b"pub fn value() -> u8 { 97 }\n");
    let authority = AuthorityRoot::new("production-process-race");
    let executable = std::env::current_exe().unwrap();
    let outcome_a = authority.parent.join("outcome-a");
    let outcome_b = authority.parent.join("outcome-b");
    let spawn = |outcome: &Path| {
        Command::new(&executable)
            .args([
                "--ignored",
                "--exact",
                "production_child_race_attempt",
                "--nocapture",
            ])
            .env("HUL_ROUTINE_PRODUCTION_CHILD", "1")
            .env("HUL_ROUTINE_REPO", repo.root())
            .env("HUL_ROUTINE_AUTHORITY", authority.path())
            .env("HUL_ROUTINE_OUTCOME", outcome)
            .spawn()
            .unwrap()
    };
    let mut child_a = spawn(&outcome_a);
    let mut child_b = spawn(&outcome_b);
    assert!(child_a.wait().unwrap().success());
    assert!(child_b.wait().unwrap().success());
    let outcomes = [
        fs::read_to_string(outcome_a).unwrap(),
        fs::read_to_string(outcome_b).unwrap(),
    ];
    assert_eq!(
        outcomes.iter().filter(|value| *value == "complete").count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|value| value.contains("replayed"))
            .count(),
        1,
        "outcomes={outcomes:?}"
    );
}

#[test]
fn forged_and_valid_reuse_concurrency_is_order_independent_and_single_transition() {
    for forged_first in [true, false] {
        let repo = TempRepo::new(if forged_first {
            "production-forged-valid-race-forged-first"
        } else {
            "production-forged-valid-race-valid-first"
        });
        repo.write(".git/info/exclude", b"target/\n");
        fs::create_dir_all(repo.root().join("target/routine/compile")).unwrap();
        repo.write("src/lib.rs", b"pub fn value() -> u8 { 103 }\n");
        let fixture = fixture_from_repo(repo, "routine-production-race");
        let authority = AuthorityRoot::new(if forged_first {
            "production-forged-valid-race-forged-first"
        } else {
            "production-forged-valid-race-valid-first"
        });
        let first = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap();
        let valid_reuse = first.reuse_artifacts().to_vec();
        let forged_reuse = foreign_reuse_for_same_request(
            &fixture,
            if forged_first {
                "production-forged-valid-race-foreign-a"
            } else {
                "production-forged-valid-race-foreign-b"
            },
        );
        assert_ne!(valid_reuse[0], forged_reuse[0]);
        let valid_path = authority.parent.join("valid-reuse-artifact");
        let forged_path = authority.parent.join("forged-reuse-artifact");
        fs::write(&valid_path, &valid_reuse[0]).unwrap();
        fs::write(&forged_path, &forged_reuse[0]).unwrap();
        let before_target = fixture.repo.tree();
        let before_status = fixture.repo.status();

        let executable = std::env::current_exe().unwrap();
        let outcome_first = authority.parent.join("ordered-outcome-first");
        let outcome_second = authority.parent.join("ordered-outcome-second");
        let spawn = |reuse: &Path, outcome: &Path| {
            Command::new(&executable)
                .args([
                    "--ignored",
                    "--exact",
                    "production_child_race_attempt",
                    "--nocapture",
                ])
                .env("HUL_ROUTINE_PRODUCTION_CHILD", "1")
                .env("HUL_ROUTINE_REPO", fixture.repo.root())
                .env("HUL_ROUTINE_AUTHORITY", authority.path())
                .env("HUL_ROUTINE_OUTCOME", outcome)
                .env("HUL_ROUTINE_REUSE", reuse)
                .spawn()
                .unwrap()
        };
        let (first_path, second_path) = if forged_first {
            (&forged_path, &valid_path)
        } else {
            (&valid_path, &forged_path)
        };
        let mut first_child = spawn(first_path, &outcome_first);
        let mut second_child = spawn(second_path, &outcome_second);
        assert!(first_child.wait().unwrap().success());
        assert!(second_child.wait().unwrap().success());
        let outcomes = [
            fs::read_to_string(&outcome_first).unwrap(),
            fs::read_to_string(&outcome_second).unwrap(),
        ];
        assert_eq!(
            outcomes.iter().filter(|value| *value == "complete").count(),
            1,
            "outcomes={outcomes:?}"
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|value| *value == "mediator-production-reuse-not-authenticated")
                .count(),
            1,
            "outcomes={outcomes:?}"
        );
        assert_eq!(authority_cardinalities(&authority), (1, 1, 2));
        assert_eq!(fixture.repo.tree(), before_target);
        assert_eq!(fixture.repo.status(), before_status);
        let reopened = ProductionRoutineIssuer::open(authority.path()).unwrap();
        assert!(
            reopened
                .pending_recovery(&fixture.context, &fixture.plan, &prepared(&fixture))
                .unwrap()
                .is_none()
        );
        let exact = mediate(&authority, &fixture, prepared(&fixture), valid_reuse).unwrap();
        assert_eq!(
            exact.nodes()[0].disposition(),
            RoutineNodeDisposition::Reused
        );
    }
}

#[test]
fn concurrent_reuse_at_consumed_grant_capacity_publishes_one_valid_max_state() {
    let repo = TempRepo::new("production-capacity-process-race");
    repo.write(".git/info/exclude", b"target/\n");
    fs::create_dir_all(repo.root().join("target/routine/compile")).unwrap();
    repo.write("src/lib.rs", b"pub fn value() -> u8 { 101 }\n");
    let fixture = fixture_from_repo(repo, "routine-production-race");
    let authority = AuthorityRoot::new("production-capacity-process-race");
    let first = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap();
    let reuse_path = authority.parent.join("reuse-artifact");
    fs::write(&reuse_path, &first.reuse_artifacts()[0]).unwrap();
    let (_, consumed_limit) = ProductionRoutineIssuer::test_capacity_limits();
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    issuer.test_seed_capacity(1, consumed_limit - 1).unwrap();
    drop(issuer);
    drop(first);

    let executable = std::env::current_exe().unwrap();
    let outcome_a = authority.parent.join("capacity-outcome-a");
    let outcome_b = authority.parent.join("capacity-outcome-b");
    let spawn = |outcome: &Path| {
        Command::new(&executable)
            .args([
                "--ignored",
                "--exact",
                "production_child_race_attempt",
                "--nocapture",
            ])
            .env("HUL_ROUTINE_PRODUCTION_CHILD", "1")
            .env("HUL_ROUTINE_REPO", fixture.repo.root())
            .env("HUL_ROUTINE_AUTHORITY", authority.path())
            .env("HUL_ROUTINE_OUTCOME", outcome)
            .env("HUL_ROUTINE_REUSE", &reuse_path)
            .spawn()
            .unwrap()
    };
    let mut child_a = spawn(&outcome_a);
    let mut child_b = spawn(&outcome_b);
    assert!(child_a.wait().unwrap().success());
    assert!(child_b.wait().unwrap().success());
    let outcomes = [
        fs::read_to_string(outcome_a).unwrap(),
        fs::read_to_string(outcome_b).unwrap(),
    ];
    assert_eq!(
        outcomes.iter().filter(|value| *value == "complete").count(),
        1,
        "outcomes={outcomes:?}"
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|value| *value == "routine-production-authority-capacity-exhausted")
            .count(),
        1,
        "outcomes={outcomes:?}"
    );
    assert_eq!(authority_cardinalities(&authority), (1, 1, consumed_limit));
    let max_state = authority_state(&authority);
    let reopened = ProductionRoutineIssuer::open(authority.path()).unwrap();
    assert!(
        reopened
            .pending_recovery(&fixture.context, &fixture.plan, &prepared(&fixture))
            .unwrap()
            .is_none()
    );
    let replay = mediate(&authority, &fixture, prepared(&fixture), Vec::new()).unwrap_err();
    assert_eq!(
        replay.cause(),
        "routine-production-semantic-effect-replayed"
    );
    assert_eq!(authority_state(&authority), max_state);
}

fn tree(root: &Path) -> BTreeMap<String, String> {
    let mut rows = BTreeMap::new();
    if root.exists() {
        visit(root, root, &mut rows);
    }
    rows
}

fn visit(root: &Path, current: &Path, rows: &mut BTreeMap<String, String>) {
    let mut entries = fs::read_dir(current)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let metadata = fs::symlink_metadata(&path).unwrap();
        if metadata.is_dir() {
            rows.insert(relative, "directory".to_owned());
            visit(root, &path, rows);
        } else if metadata.file_type().is_symlink() {
            rows.insert(relative, "symlink".to_owned());
        } else if metadata.is_file() {
            rows.insert(relative, format!("file:{}", sha(&fs::read(&path).unwrap())));
        } else {
            rows.insert(relative, "special".to_owned());
        }
    }
}
