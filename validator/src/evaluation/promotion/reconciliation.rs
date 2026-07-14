impl PromotionDecision {
    pub(crate) fn reconcile<A: PromotionReviewAuthority>(
        baseline: &EvaluationRun,
        candidate: &EvaluationRun,
        review: &PromotionReview,
        authority: &mut A,
    ) -> Self {
        let mut reasons = Vec::new();
        let pair_is_current = runs_comparable(baseline, candidate)
            && run_revalidates(baseline)
            && run_revalidates(candidate);
        if !pair_is_current {
            reasons.push("evaluation-pair-not-comparable".to_owned());
        }

        let principal_is_independent = valid_identifier(&review.reviewer_id)
            && valid_identifier(&review.authority_id)
            && review.reviewer_id != review.authority_id
            && review.issuance_session_id != baseline.execution_session_id
            && review.issuance_session_id != candidate.execution_session_id
            && baseline.execution_session_id != candidate.execution_session_id
            && !review_principal_conflicts(baseline, candidate, &review.reviewer_id)
            && !review_principal_conflicts(baseline, candidate, &review.authority_id);
        if !principal_is_independent {
            reasons.push("evaluation-review-not-independent".to_owned());
        }

        let evidence_is_complete = review_evidence_is_complete(
            &review.representative_journey,
            &review.rollback_evidence,
            &review.reviewed_artifacts,
        );
        if !evidence_is_complete {
            reasons.push("evaluation-review-evidence-invalid".to_owned());
        }

        let (current_context, current_candidate) = authority.current_binding();
        let current_context = current_context.to_owned();
        let current_candidate = current_candidate.to_owned();
        let authority_is_current = authority.authority_id() == review.authority_id
            && authority.reviewer_id() == review.reviewer_id
            && authority.review_session_id() == review.issuance_session_id
            && current_context == review.live_context_id
            && current_candidate == review.candidate_id;
        if !authority_is_current {
            reasons.push("evaluation-review-authority-stale".to_owned());
        }

        let expected_binding = promotion_review_binding(PromotionReviewBinding {
            baseline,
            candidate,
            reviewer_id: &review.reviewer_id,
            authority_id: &review.authority_id,
            issuance_session_id: &review.issuance_session_id,
            representative_journey: &review.representative_journey,
            rollback_evidence: &review.rollback_evidence,
            reviewed_artifacts: &review.reviewed_artifacts,
        });
        let expected_review_id = digest(
            format!(
                "promotion-review|{}|{}",
                expected_binding, review.attestation_sha256
            )
            .as_bytes(),
        );
        let binding_is_current = review_matches_runs(review, baseline, candidate)
            && valid_sha256(&review.attestation_sha256)
            && review.binding_sha256 == expected_binding
            && review.review_id == expected_review_id;
        if !binding_is_current {
            reasons.push("evaluation-review-binding-stale-or-substituted".to_owned());
        }

        if pair_is_current
            && principal_is_independent
            && evidence_is_complete
            && authority_is_current
            && binding_is_current
        {
            let attestation_accepted = authority.verify_and_consume(
                &review.binding_sha256,
                &review.reviewer_id,
                &review.review_id,
                &review.attestation_sha256,
            );
            let authority_remained_current = authority.authority_id() == review.authority_id
                && authority.reviewer_id() == review.reviewer_id
                && authority.review_session_id() == review.issuance_session_id
                && authority.current_binding()
                    == (current_context.as_str(), current_candidate.as_str());
            if !attestation_accepted || !authority_remained_current {
                reasons.push("evaluation-review-attestation-invalid-or-replayed".to_owned());
            }
        }

        let baseline_by_id = baseline
            .results
            .iter()
            .map(|result| (result.task_id.as_str(), result))
            .collect::<BTreeMap<_, _>>();
        let candidate_by_id = candidate
            .results
            .iter()
            .map(|result| (result.task_id.as_str(), result))
            .collect::<BTreeMap<_, _>>();
        if baseline_by_id.keys().collect::<Vec<_>>() != candidate_by_id.keys().collect::<Vec<_>>() {
            reasons.push("evaluation-task-set-drift".to_owned());
        }
        let baseline_failures = baseline
            .results
            .iter()
            .filter(|result| result.outcome != BehaviorOutcome::Passed)
            .count();
        let candidate_failures = candidate
            .results
            .iter()
            .filter(|result| result.outcome != BehaviorOutcome::Passed)
            .count();
        if candidate_failures >= baseline_failures {
            reasons.push("evaluation-no-representative-behavior-improvement".to_owned());
        }
        if candidate
            .results
            .iter()
            .any(|result| result.representative && result.outcome != BehaviorOutcome::Passed)
        {
            reasons.push("evaluation-representative-regression".to_owned());
        }
        for (task_id, candidate_result) in &candidate_by_id {
            let Some(baseline_result) = baseline_by_id.get(task_id) else {
                continue;
            };
            if u128::from(candidate_result.score_earned)
                * u128::from(baseline_result.score_possible)
                < u128::from(baseline_result.score_earned)
                    * u128::from(candidate_result.score_possible)
                || (baseline_result.outcome == BehaviorOutcome::Passed
                    && candidate_result.outcome != BehaviorOutcome::Passed)
            {
                reasons.push(format!("evaluation-task-regressed:{task_id}"));
            }
            for control in PerturbationControl::REQUIRED {
                if !candidate_result.passed_perturbations.contains(&control) {
                    reasons.push(format!(
                        "evaluation-reward-hacking-control-missing:{}",
                        control.label()
                    ));
                }
            }
        }
        let (baseline_earned, baseline_possible) = totals(&baseline.results);
        let (candidate_earned, candidate_possible) = totals(&candidate.results);
        if u128::from(candidate_earned) * u128::from(baseline_possible)
            <= u128::from(baseline_earned) * u128::from(candidate_possible)
        {
            reasons.push("evaluation-score-not-improved".to_owned());
        }
        reasons.sort();
        reasons.dedup();
        Self {
            baseline_candidate_id: baseline.candidate_id.clone(),
            candidate_id: candidate.candidate_id.clone(),
            reviewer_id: review.reviewer_id.clone(),
            status: if reasons.is_empty() {
                PromotionStatus::ImprovementCandidate
            } else {
                PromotionStatus::Rejected
            },
            baseline_run_sha256: baseline.run_sha256.clone(),
            candidate_run_sha256: candidate.run_sha256.clone(),
            reasons,
            claim_ceiling: "improvement_candidate_not_product_completion".to_owned(),
        }
    }
}
