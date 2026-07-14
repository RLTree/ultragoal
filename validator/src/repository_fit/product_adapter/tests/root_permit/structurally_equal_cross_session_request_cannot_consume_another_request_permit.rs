use super::*;

#[test]
pub(crate) fn structurally_equal_cross_session_request_cannot_consume_another_request_permit() {
    let fixture = Fixture::new("cross-session");
    let context = fixture.context();
    let request = fixture.request(&context);
    let substituted = fixture.request(&context);
    assert_eq!(request.plan.plan_sha256, substituted.plan.plan_sha256);
    assert_ne!(request.request_id, substituted.request_id);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("cross-session"),
        )
        .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        substituted,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyPermitInvalid);
    assert!(!failure.effect_started());
    let (_substituted, permit, lease) = failure.into_pre_effect().unwrap().into_parts();
    assert_eq!(permit_seal_stage_for_test(&request), 0);
    let outcome = expect_apply_ok(apply_with_root_permit(&context, request, permit, lease, 10));
    assert_eq!(outcome.status(), "applied");
}

#[test]
pub(crate) fn concurrent_identical_contenders_have_one_atomic_winner() {
    let fixture = Fixture::new("concurrent-one-winner");
    fixture.install_all();
    let context = fixture.context();
    let request = fixture.request(&context);
    assert!(request.plan.mutations.is_empty());
    let duplicate_request = request.duplicate_for_test();
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("concurrent"),
        )
        .unwrap();
    let (duplicate_permit, duplicate_lease) = duplicate_authorization_for_test(
        &permit,
        &duplicate_request,
        fixture.effects(&duplicate_request),
    );
    let start = Arc::new(Barrier::new(3));
    let first_start = Arc::clone(&start);
    let second_start = Arc::clone(&start);
    let first_context = context.clone();
    let second_context = context.clone();
    let first = std::thread::spawn(move || {
        first_start.wait();
        apply_with_root_permit(&first_context, request, Some(permit), Some(lease), 10)
    });
    let second = std::thread::spawn(move || {
        second_start.wait();
        apply_with_root_permit(
            &second_context,
            duplicate_request,
            Some(duplicate_permit),
            Some(duplicate_lease),
            10,
        )
    });
    start.wait();
    let results = [first.join().unwrap(), second.join().unwrap()];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    let loser = results
        .into_iter()
        .find_map(Result::err)
        .expect("one contender must lose");
    assert_eq!(loser.error().id(), AdapterErrorId::ApplyPermitReplayed);
    assert!(loser.effect_started());
}

pub(crate) fn assert_request_tamper_rejected(
    label: &str,
    mutate: impl FnOnce(&mut OpaqueFitApplyRequest),
) {
    let fixture = Fixture::new(label);
    let context = fixture.context();
    let mut request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce(label),
        )
        .unwrap();
    mutate(&mut request);
    let before = snapshot(&fixture.root);
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert!(!failure.effect_started(), "{label}");
    assert_ne!(
        failure.error().id(),
        AdapterErrorId::ApplyRolledBack,
        "{label}"
    );
    assert_eq!(snapshot(&fixture.root), before, "{label}");
}

#[test]
pub(crate) fn accepted_plan_desired_state_source_set_modes_and_paths_are_individually_bound() {
    assert_request_tamper_rejected("accepted-digest", |request| {
        request.accepted_plan_sha256 = digest(b"substituted accepted plan");
    });
    assert_request_tamper_rejected("plan-mutation", |request| {
        request.plan.mutations[0].replacement.push(b'x');
    });
    assert_request_tamper_rejected("desired-state", |request| {
        request.desired.files[0].bytes.push(b'x');
    });
    assert_request_tamper_rejected("source-row", |request| {
        request.authority.rows[0].sha256 = digest(b"substituted template source");
    });
    assert_request_tamper_rejected("catalog", |request| {
        request.authority.catalog_sha256 = digest(b"substituted catalog");
    });
    assert_request_tamper_rejected("mode", |request| {
        let path = request.plan.mutations[0].path.as_str().to_owned();
        request.unix_modes.insert(path, 0o600);
    });
    assert_request_tamper_rejected("case-alias", |request| {
        request.plan.mutations[0].path = CanonicalPath::parse("Agents.md").unwrap();
    });
}

#[test]
pub(crate) fn request_identity_context_root_binding_and_canonical_record_are_individually_bound() {
    assert_request_tamper_rejected("request-id", |request| {
        request.request_id = digest(b"substituted request identity");
    });
    assert_request_tamper_rejected("context-id", |request| {
        request.context_id = digest(b"substituted context identity");
    });
    assert_request_tamper_rejected("candidate-id", |request| {
        request.candidate_id = digest(b"substituted candidate identity");
    });
    assert_request_tamper_rejected("root-binding", |request| {
        request.root_binding = digest(b"substituted descriptor root binding");
    });
    assert_request_tamper_rejected("canonical-plan-record", |request| {
        request.plan_record_bytes.push(b' ');
    });
}

#[test]
pub(crate) fn target_projection_dimensions_are_individually_bound() {
    assert_request_tamper_rejected("target-context", |request| {
        request.target.context_id = digest(b"substituted target context");
    });
    assert_request_tamper_rejected("repository-root-id", |request| {
        request.target.repository_root_id = digest(b"substituted repository root");
    });
    assert_request_tamper_rejected("worktree-root-id", |request| {
        request.target.worktree_root_id = digest(b"substituted worktree root");
    });
    assert_request_tamper_rejected("target-candidate-id", |request| {
        request.target.candidate.candidate_id = digest(b"substituted target candidate");
    });
}
