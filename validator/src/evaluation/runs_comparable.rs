fn runs_comparable(baseline: &EvaluationRun, candidate: &EvaluationRun) -> bool {
    baseline.live_context_id == candidate.live_context_id
        && baseline.candidate_id != candidate.candidate_id
        && baseline.spec_id == candidate.spec_id
        && baseline.task_set_sha256 == candidate.task_set_sha256
        && baseline.execution_session_id != candidate.execution_session_id
        && baseline
            .results
            .iter()
            .map(|result| result.task_id.as_str())
            .eq(candidate
                .results
                .iter()
                .map(|result| result.task_id.as_str()))
}

fn run_revalidates(run: &EvaluationRun) -> bool {
    let mut task_ids = BTreeSet::new();
    let mut fixture_ids = BTreeSet::new();
    valid_sha256(&run.live_context_id)
        && valid_sha256(&run.candidate_id)
        && valid_identifier(&run.spec_id)
        && valid_sha256(&run.task_set_sha256)
        && valid_sha256(&run.execution_session_id)
        && run.spec_sha256
            == digest(
                format!(
                    "{}|{}|{}|{}",
                    run.live_context_id, run.candidate_id, run.spec_id, run.task_set_sha256
                )
                .as_bytes(),
            )
        && !run.results.is_empty()
        && run.results.iter().all(|result| {
            task_ids.insert(result.task_id.as_str())
                && fixture_ids.insert(result.fixture_id.as_str())
                && valid_identifier(&result.task_id)
                && valid_identifier(&result.fixture_id)
                && valid_identifier(&result.causal_code)
                && valid_sha256(&result.artifact_digest_sha256)
                && valid_identifier(&result.producer_id)
                && valid_identifier(&result.observer_id)
                && valid_identifier(&result.scorer_id)
                && valid_identifier(&result.independent_grader_id)
                && result.producer_id != result.observer_id
                && result.observer_id != result.scorer_id
                && result.independent_grader_id != result.scorer_id
                && result.independent_grader_id != result.producer_id
                && result.independent_grader_id != result.observer_id
                && result.score_possible > 0
                && result.score_earned <= result.score_possible
                && result.independent_score_possible > 0
                && result.independent_score_earned <= result.independent_score_possible
                && u128::from(result.score_earned) * u128::from(result.independent_score_possible)
                    == u128::from(result.independent_score_earned)
                        * u128::from(result.score_possible)
                && result.work_units > 0
                && PerturbationControl::REQUIRED
                    .iter()
                    .all(|control| result.passed_perturbations.contains(control))
        })
        && run.run_sha256
            == run_digest_fields(
                &run.spec_sha256,
                &run.live_context_id,
                &run.candidate_id,
                &run.execution_session_id,
                &run.results,
            )
}

fn review_principal_conflicts(
    baseline: &EvaluationRun,
    candidate: &EvaluationRun,
    principal: &str,
) -> bool {
    baseline
        .results
        .iter()
        .chain(&candidate.results)
        .any(|result| {
            result.producer_id == principal
                || result.observer_id == principal
                || result.scorer_id == principal
                || result.independent_grader_id == principal
        })
}

fn review_evidence_is_complete(
    representative_journey: &BoundInput,
    rollback_evidence: &BoundInput,
    reviewed_artifacts: &[BoundInput],
) -> bool {
    if !representative_journey
        .findings("evaluation-review-journey")
        .is_empty()
        || !rollback_evidence
            .findings("evaluation-review-rollback")
            .is_empty()
        || reviewed_artifacts.is_empty()
        || reviewed_artifacts.len() > 64
        || reviewed_artifacts
            .iter()
            .any(|input| !input.findings("evaluation-reviewed-artifact").is_empty())
        || reviewed_artifacts
            .windows(2)
            .any(|pair| pair[0].relative_path >= pair[1].relative_path)
    {
        return false;
    }

    let mut paths = BTreeSet::new();
    let mut digests = BTreeSet::new();
    std::iter::once(representative_journey)
        .chain(std::iter::once(rollback_evidence))
        .chain(reviewed_artifacts)
        .all(|input| {
            paths.insert(input.relative_path.as_str())
                && digests.insert(input.digest_sha256.as_str())
        })
}

struct PromotionReviewBinding<'a> {
    baseline: &'a EvaluationRun,
    candidate: &'a EvaluationRun,
    reviewer_id: &'a str,
    authority_id: &'a str,
    issuance_session_id: &'a str,
    representative_journey: &'a BoundInput,
    rollback_evidence: &'a BoundInput,
    reviewed_artifacts: &'a [BoundInput],
}

fn promotion_review_binding(binding: PromotionReviewBinding<'_>) -> String {
    let PromotionReviewBinding {
        baseline,
        candidate,
        reviewer_id,
        authority_id,
        issuance_session_id,
        representative_journey,
        rollback_evidence,
        reviewed_artifacts,
    } = binding;
    let artifacts = reviewed_artifacts
        .iter()
        .map(BoundInput::digest_fragment)
        .collect::<Vec<_>>()
        .join("\n");
    digest(
        format!(
            "promotion-review-v1|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            reviewer_id,
            authority_id,
            issuance_session_id,
            candidate.live_context_id,
            baseline.candidate_id,
            candidate.candidate_id,
            baseline.run_sha256,
            candidate.run_sha256,
            baseline.spec_id,
            candidate.spec_id,
            baseline.spec_sha256,
            candidate.spec_sha256,
            baseline.task_set_sha256,
            candidate.task_set_sha256,
            baseline.execution_session_id,
            candidate.execution_session_id,
            representative_journey.digest_fragment(),
            format!("{}|{}", rollback_evidence.digest_fragment(), artifacts),
        )
        .as_bytes(),
    )
}

fn review_matches_runs(
    review: &PromotionReview,
    baseline: &EvaluationRun,
    candidate: &EvaluationRun,
) -> bool {
    review.live_context_id == candidate.live_context_id
        && review.baseline_candidate_id == baseline.candidate_id
        && review.candidate_id == candidate.candidate_id
        && review.baseline_run_sha256 == baseline.run_sha256
        && review.candidate_run_sha256 == candidate.run_sha256
        && review.baseline_spec_id == baseline.spec_id
        && review.candidate_spec_id == candidate.spec_id
        && review.baseline_spec_sha256 == baseline.spec_sha256
        && review.candidate_spec_sha256 == candidate.spec_sha256
        && review.task_set_sha256 == candidate.task_set_sha256
        && review.task_set_sha256 == baseline.task_set_sha256
        && review.baseline_execution_session_id == baseline.execution_session_id
        && review.candidate_execution_session_id == candidate.execution_session_id
}
