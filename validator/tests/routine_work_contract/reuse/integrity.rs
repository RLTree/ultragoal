use super::super::routine_work::{
    ReuseDecision, ReuseExpectation, ReuseMiss, ReuseReceipt, RoutineErrorId, assess_reuse,
    set_test_live_authority_hook,
};
use super::super::support::{TempRepo, sha};
use super::common::{
    authority_context, authority_plan, capture_bytes, capture_guard, capture_receipt, expectation,
    issue_execution, syntax_evidence,
};

#[cfg(target_os = "macos")]
#[test]
fn dependency_results_can_only_come_from_opaque_executed_or_reused_work() {
    let _capture = capture_guard();
    let repo = TempRepo::new("reuse-dependencies");
    let (context, plan) = authority_plan(&repo);
    let syntax = expectation(&context, &plan, "syntax", Vec::new());
    let syntax = issue_execution(&repo, &context, &syntax, b"syntax");
    let passed = syntax.work().dependency_result(&context).unwrap();
    ReuseExpectation::for_check(
        &context,
        &plan,
        plan.check("compile").unwrap(),
        vec![passed.clone()],
        "routine",
    )
    .unwrap();
    for dependencies in [Vec::new(), vec![passed.clone(), passed.clone()]] {
        assert_eq!(
            ReuseExpectation::for_check(
                &context,
                &plan,
                plan.check("compile").unwrap(),
                dependencies,
                "routine",
            )
            .unwrap_err()
            .id(),
            RoutineErrorId::InvalidReceipt
        );
    }
    assert_eq!(
        ReuseExpectation::for_check(
            &context,
            &plan,
            plan.check("unit").unwrap(),
            vec![passed],
            "routine",
        )
        .unwrap_err()
        .id(),
        RoutineErrorId::InvalidReceipt
    );
}

#[cfg(target_os = "macos")]
#[test]
fn expectation_and_dependency_result_revalidate_live_authority() {
    let _capture = capture_guard();
    let expectation_repo = TempRepo::new("expectation-live-mutation");
    let (expectation_context, expectation_plan) = authority_plan(&expectation_repo);
    let root = expectation_repo.root().to_path_buf();
    set_test_live_authority_hook(move || {
        std::fs::write(root.join("src/lib.rs"), b"mutated during expectation\n").unwrap();
    });
    assert_eq!(
        ReuseExpectation::for_check(
            &expectation_context,
            &expectation_plan,
            expectation_plan.check("syntax").unwrap(),
            Vec::new(),
            "routine",
        )
        .unwrap_err()
        .id(),
        RoutineErrorId::ConcurrentMutation
    );

    let dependency_repo = TempRepo::new("dependency-live-mutation");
    let (dependency_context, dependency_plan) = authority_plan(&dependency_repo);
    let expectation = expectation(&dependency_context, &dependency_plan, "syntax", Vec::new());
    let execution = issue_execution(
        &dependency_repo,
        &dependency_context,
        &expectation,
        b"syntax",
    );
    let root = dependency_repo.root().to_path_buf();
    set_test_live_authority_hook(move || {
        std::fs::write(
            root.join("src/lib.rs"),
            b"mutated during dependency result\n",
        )
        .unwrap();
    });
    assert_eq!(
        execution
            .work()
            .dependency_result(&dependency_context)
            .unwrap_err()
            .id(),
        RoutineErrorId::ConcurrentMutation
    );
}

#[cfg(target_os = "macos")]
#[test]
fn a_check_from_another_plan_cannot_be_substituted() {
    let first_repo = TempRepo::new("reuse-first-plan");
    let second_repo = TempRepo::new("reuse-second-plan");
    let (first_context, first) = authority_plan(&first_repo);
    let (second_context, second) = authority_plan(&second_repo);
    assert_eq!(
        ReuseExpectation::for_check(
            &first_context,
            &first,
            second.check("syntax").unwrap(),
            Vec::new(),
            "routine",
        )
        .unwrap_err()
        .id(),
        RoutineErrorId::InvalidReceipt
    );
    let first_expectation = expectation(&first_context, &first, "syntax", Vec::new());
    let first_execution =
        issue_execution(&first_repo, &first_context, &first_expectation, b"syntax");
    let first_dependency = first_execution
        .work()
        .dependency_result(&first_context)
        .unwrap();
    assert_eq!(
        ReuseExpectation::for_check(
            &second_context,
            &second,
            second.check("compile").unwrap(),
            vec![first_dependency],
            "routine",
        )
        .unwrap_err()
        .id(),
        RoutineErrorId::InvalidReceipt
    );
}

#[cfg(target_os = "macos")]
#[test]
fn captured_receipt_parser_rejects_unknown_duplicate_noncanonical_and_oversized_rows() {
    let _capture = capture_guard();
    let repo = TempRepo::new("reuse-canonical");
    let (context, plan) = authority_plan(&repo);
    let (_, execution, _, _) = syntax_evidence(&repo, &context, &plan);
    let canonical = String::from_utf8(execution.receipt_json().to_vec()).unwrap();
    for (index, invalid) in [
        canonical.replacen('{', "{\"unknown\":true,", 1),
        canonical.replacen('{', "{\"schema_version\":\"VerifiedReuse-v2\",", 1),
        format!(" {canonical}"),
    ]
    .into_iter()
    .enumerate()
    {
        let artifact = capture_bytes(
            &repo,
            &context,
            invalid.as_bytes(),
            &format!("invalid-{index}"),
        )
        .unwrap();
        assert_eq!(
            ReuseReceipt::from_captured(&artifact).unwrap_err().id(),
            RoutineErrorId::InvalidReceipt
        );
    }
    let oversized = vec![b' '; 4 * 1024 * 1024 + 1];
    assert!(capture_bytes(&repo, &context, &oversized, "oversized").is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn unknown_dependency_row_misses_and_duplicate_json_row_is_rejected() {
    let _capture = capture_guard();
    let repo = TempRepo::new("reuse-row-integrity");
    let (context, plan) = authority_plan(&repo);
    let (expectation, execution, _, observed) = syntax_evidence(&repo, &context, &plan);
    let canonical = String::from_utf8(execution.receipt_json().to_vec()).unwrap();
    let digest = sha(b"unknown");
    let unknown = canonical.replacen(
        "\"dependency_results\":{}",
        &format!("\"dependency_results\":{{\"unknown\":\"{digest}\"}}"),
        1,
    );
    let unknown = capture_receipt(&repo, &context, unknown.as_bytes(), "unknown-row");
    assert_eq!(
        assess_reuse(&context, &expectation, &unknown, &observed).unwrap(),
        ReuseDecision::Miss(ReuseMiss::DependencyResult)
    );
    let duplicate = canonical.replacen(
        "\"dependency_results\":{}",
        &format!("\"dependency_results\":{{\"unknown\":\"{digest}\",\"unknown\":\"{digest}\"}}"),
        1,
    );
    let artifact = capture_bytes(&repo, &context, duplicate.as_bytes(), "duplicate-row").unwrap();
    assert_eq!(
        ReuseReceipt::from_captured(&artifact).unwrap_err().id(),
        RoutineErrorId::InvalidReceipt
    );
}

#[cfg(target_os = "macos")]
#[test]
fn mutation_after_initial_reuse_validation_cannot_return_hit() {
    let _capture = capture_guard();
    let repo = TempRepo::new("reuse-live-checkpoint");
    let (context, plan) = authority_plan(&repo);
    let (expectation, _, receipt, observed) = syntax_evidence(&repo, &context, &plan);
    let root = repo.root().to_path_buf();
    set_test_live_authority_hook(move || {
        std::fs::write(root.join("src/lib.rs"), b"mutated during reuse\n").unwrap();
    });
    assert_eq!(
        assess_reuse(&context, &expectation, &receipt, &observed)
            .unwrap_err()
            .id(),
        RoutineErrorId::ConcurrentMutation
    );
}

#[cfg(target_os = "macos")]
#[test]
fn mutation_after_observation_is_revalidated_before_reuse_decision() {
    let _capture = capture_guard();
    let repo = TempRepo::new("reuse-race");
    let (context, plan) = authority_plan(&repo);
    let (expectation, _, receipt, observed) = syntax_evidence(&repo, &context, &plan);
    repo.write("src/lib.rs", b"pub fn value() -> u8 { 91 }\n");
    let error = assess_reuse(&context, &expectation, &receipt, &observed).unwrap_err();
    assert_eq!(error.id(), RoutineErrorId::ConcurrentMutation);
    let current = authority_context(&repo);
    assert_eq!(
        assess_reuse(&current, &expectation, &receipt, &observed).unwrap(),
        ReuseDecision::Miss(ReuseMiss::Candidate)
    );
}
