use super::*;

#[test]
pub(crate) fn partial_duplicate_unknown_reordered_unobserved_and_cross_session_outcomes_fail_closed()
 {
    let fixture = dirty_fixture("adapter-invalid-outcomes");
    let before_tree = fixture.repo.tree();
    let before_status = fixture.repo.status();

    let (partial_authority, partial_intents) = mediation(&fixture);
    let mut partial = skipped_outcomes(partial_intents);
    partial.pop();
    assert_eq!(
        reconcile_routine_execution(&fixture.context, &fixture.plan, partial_authority, partial,)
            .unwrap_err()
            .cause(),
        "adapter-outcome-cardinality-inexact"
    );

    let (duplicate_authority, duplicate_intents) = mediation(&fixture);
    let first_id = duplicate_intents[0].intent().intent_id().to_owned();
    let first_node = duplicate_intents[0].intent().node_id().to_owned();
    let mut duplicate = skipped_outcomes(duplicate_intents);
    let duplicate_row = duplicate
        .remove(1)
        .test_with_intent(first_id, 1, first_node);
    duplicate.insert(1, duplicate_row);
    assert_eq!(
        reconcile_routine_execution(
            &fixture.context,
            &fixture.plan,
            duplicate_authority,
            duplicate,
        )
        .unwrap_err()
        .cause(),
        "adapter-outcome-duplicated"
    );

    let (unknown_authority, unknown_intents) = mediation(&fixture);
    let first_node = unknown_intents[0].intent().node_id().to_owned();
    let mut unknown = skipped_outcomes(unknown_intents);
    let unknown_row =
        unknown
            .remove(0)
            .test_with_intent(format!("sha256:{}", "f".repeat(64)), 0, first_node);
    unknown.insert(0, unknown_row);
    assert_eq!(
        reconcile_routine_execution(&fixture.context, &fixture.plan, unknown_authority, unknown,)
            .unwrap_err()
            .cause(),
        "adapter-outcome-unknown"
    );

    let (reordered_authority, reordered_intents) = mediation(&fixture);
    let mut reordered = skipped_outcomes(reordered_intents);
    reordered.swap(0, 1);
    assert_eq!(
        reconcile_routine_execution(
            &fixture.context,
            &fixture.plan,
            reordered_authority,
            reordered,
        )
        .unwrap_err()
        .cause(),
        "adapter-outcome-order-invalid"
    );

    let (unobserved_authority, unobserved_intents) = mediation(&fixture);
    let mut unobserved = skipped_outcomes(unobserved_intents);
    let unobserved_row = unobserved.remove(0).test_with_observed(false);
    unobserved.insert(0, unobserved_row);
    assert_eq!(
        reconcile_routine_execution(
            &fixture.context,
            &fixture.plan,
            unobserved_authority,
            unobserved,
        )
        .unwrap_err()
        .cause(),
        "adapter-outcome-unobserved"
    );

    let (first_authority, first_intents) = mediation(&fixture);
    let cross_session = skipped_outcomes(first_intents);
    let (second_authority, second_intents) = mediation(&fixture);
    assert_eq!(
        first_authority.protocol_id(),
        second_authority.protocol_id()
    );
    assert_ne!(first_authority.request_id(), second_authority.request_id());
    let second_request_id = second_authority.request_id().to_owned();
    let second_protocol_id = second_authority.protocol_id().to_owned();
    let copied_ids = cross_session
        .into_iter()
        .map(|outcome| {
            outcome.test_with_identity(second_request_id.clone(), second_protocol_id.clone())
        })
        .collect();
    assert_eq!(
        reconcile_routine_execution(
            &fixture.context,
            &fixture.plan,
            second_authority,
            copied_ids,
        )
        .unwrap_err()
        .cause(),
        "adapter-outcome-session-mismatch"
    );
    drop(second_intents);

    let (skipped_authority, skipped_intents) = mediation(&fixture);
    let skipped = skipped_outcomes(skipped_intents);
    assert_eq!(
        reconcile_routine_execution(&fixture.context, &fixture.plan, skipped_authority, skipped,)
            .unwrap_err()
            .cause(),
        "adapter-outcome-not-complete"
    );

    let (failed_authority, failed_intents) = mediation(&fixture);
    let failed = failed_outcomes(failed_intents);
    assert_eq!(
        reconcile_routine_execution(&fixture.context, &fixture.plan, failed_authority, failed,)
            .unwrap_err()
            .cause(),
        "adapter-outcome-not-complete"
    );

    let replayed = request(&fixture);
    replayed.test_mark_transitioned().unwrap();
    assert_eq!(
        mediation_error(begin_routine_mediation(
            &fixture.context,
            &fixture.plan,
            replayed,
        ))
        .cause(),
        "adapter-effect-request-replayed"
    );
    assert_eq!(fixture.repo.tree(), before_tree);
    assert_eq!(fixture.repo.status(), before_status);
}

#[cfg(target_os = "macos")]
#[test]
pub(crate) fn genuine_ordered_outcomes_delegate_to_exact_report_reconciliation() {
    let _capture = capture_guard();
    let fixture = dirty_fixture("adapter-complete");
    let (authority, intents) = mediation(&fixture);
    assert_eq!(authority.requested_result_scope(), "routine");
    let execution_result_scope = authority.execution_result_scope().to_owned();
    let outcomes = executed_outcomes(&fixture, intents);
    let report =
        reconcile_routine_execution(&fixture.context, &fixture.plan, authority, outcomes).unwrap();
    assert_eq!(report.status(), ReportStatus::CompleteExecution);
    assert_eq!(report.context_id(), fixture.context.context_id());
    assert_eq!(report.candidate_id(), fixture.plan.binding().candidate_id());
    assert_eq!(report.result_scope(), execution_result_scope);
    assert!(report.result_scope().starts_with("rma:"));
    assert_eq!(report.selected().len(), 3);
    assert!(report.reused().is_empty());
    assert!(report.support_limit().contains("no claim decision"));
}
