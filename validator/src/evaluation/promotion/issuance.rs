pub(crate) struct PromotionReviewEvidence {
    pub representative_journey: BoundInput,
    pub rollback_evidence: BoundInput,
    pub reviewed_artifacts: Vec<BoundInput>,
}

impl PromotionReview {
    pub(crate) fn issue<A: PromotionReviewAuthority>(
        baseline: &EvaluationRun,
        candidate: &EvaluationRun,
        evidence: PromotionReviewEvidence,
        authority: &mut A,
    ) -> Result<Self, EvaluationError> {
        let PromotionReviewEvidence {
            representative_journey,
            rollback_evidence,
            mut reviewed_artifacts,
        } = evidence;
        let authority_id = authority.authority_id().to_owned();
        let reviewer_id = authority.reviewer_id().to_owned();
        let issuance_session_id = authority.review_session_id().to_owned();
        let (authority_context, authority_candidate) = authority.current_binding();
        let authority_context = authority_context.to_owned();
        let authority_candidate = authority_candidate.to_owned();
        reviewed_artifacts.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

        if !runs_comparable(baseline, candidate)
            || !run_revalidates(baseline)
            || !run_revalidates(candidate)
            || !valid_identifier(&reviewer_id)
            || !valid_identifier(&authority_id)
            || reviewer_id == authority_id
            || !valid_sha256(&issuance_session_id)
            || authority_context != candidate.live_context_id
            || authority_candidate != candidate.candidate_id
            || issuance_session_id == baseline.execution_session_id
            || issuance_session_id == candidate.execution_session_id
            || baseline.execution_session_id == candidate.execution_session_id
            || review_principal_conflicts(baseline, candidate, &reviewer_id)
            || review_principal_conflicts(baseline, candidate, &authority_id)
            || !review_evidence_is_complete(
                &representative_journey,
                &rollback_evidence,
                &reviewed_artifacts,
            )
        {
            return Err(EvaluationError::new("evaluation-review-issuance-refused"));
        }

        let binding_sha256 = promotion_review_binding(PromotionReviewBinding {
            baseline,
            candidate,
            reviewer_id: &reviewer_id,
            authority_id: &authority_id,
            issuance_session_id: &issuance_session_id,
            representative_journey: &representative_journey,
            rollback_evidence: &rollback_evidence,
            reviewed_artifacts: &reviewed_artifacts,
        });
        let attestation_sha256 = authority.issue_attestation(&binding_sha256)?;
        let review_id =
            digest(format!("promotion-review|{binding_sha256}|{attestation_sha256}").as_bytes());
        if !valid_sha256(&attestation_sha256)
            || authority.authority_id() != authority_id
            || authority.reviewer_id() != reviewer_id
            || authority.review_session_id() != issuance_session_id
            || authority.current_binding()
                != (authority_context.as_str(), authority_candidate.as_str())
        {
            return Err(EvaluationError::new("evaluation-review-issuance-refused"));
        }

        Ok(Self {
            reviewer_id,
            authority_id,
            issuance_session_id,
            live_context_id: candidate.live_context_id.clone(),
            baseline_candidate_id: baseline.candidate_id.clone(),
            candidate_id: candidate.candidate_id.clone(),
            baseline_run_sha256: baseline.run_sha256.clone(),
            candidate_run_sha256: candidate.run_sha256.clone(),
            baseline_spec_id: baseline.spec_id.clone(),
            candidate_spec_id: candidate.spec_id.clone(),
            baseline_spec_sha256: baseline.spec_sha256.clone(),
            candidate_spec_sha256: candidate.spec_sha256.clone(),
            task_set_sha256: candidate.task_set_sha256.clone(),
            baseline_execution_session_id: baseline.execution_session_id.clone(),
            candidate_execution_session_id: candidate.execution_session_id.clone(),
            representative_journey,
            rollback_evidence,
            reviewed_artifacts,
            binding_sha256,
            review_id,
            attestation_sha256,
        })
    }

    #[cfg(test)]
    pub(crate) fn substitute_candidate_for_test(&mut self, candidate_id: impl Into<String>) {
        self.candidate_id = candidate_id.into();
    }

    #[cfg(test)]
    pub(crate) fn substitute_journey_for_test(&mut self, journey: BoundInput) {
        self.representative_journey = journey;
    }

    #[cfg(test)]
    pub(crate) fn substitute_candidate_spec_for_test(&mut self, spec_sha256: impl Into<String>) {
        self.candidate_spec_sha256 = spec_sha256.into();
    }

    #[cfg(test)]
    pub(crate) fn inject_debug_sentinels_for_test(&mut self) -> Vec<String> {
        let mut sentinels = Vec::new();
        let mut marker = |label: &str| {
            let value = format!("opaque-promotion-review-{label}-marker");
            sentinels.push(value.clone());
            value
        };

        self.reviewer_id = marker("reviewer-id");
        self.authority_id = marker("authority-id");
        self.issuance_session_id = marker("issuance-session-id");
        self.live_context_id = marker("live-context-id");
        self.baseline_candidate_id = marker("baseline-candidate-id");
        self.candidate_id = marker("candidate-id");
        self.baseline_run_sha256 = marker("baseline-run-sha256");
        self.candidate_run_sha256 = marker("candidate-run-sha256");
        self.baseline_spec_id = marker("baseline-spec-id");
        self.candidate_spec_id = marker("candidate-spec-id");
        self.baseline_spec_sha256 = marker("baseline-spec-sha256");
        self.candidate_spec_sha256 = marker("candidate-spec-sha256");
        self.task_set_sha256 = marker("task-set-sha256");
        self.baseline_execution_session_id = marker("baseline-execution-session-id");
        self.candidate_execution_session_id = marker("candidate-execution-session-id");
        self.representative_journey = BoundInput {
            relative_path: marker("representative-journey-path"),
            digest_sha256: marker("representative-journey-digest"),
            byte_length: 78_101,
            link_count: 78_111,
            kind: InputKind::Directory,
        };
        self.rollback_evidence = BoundInput {
            relative_path: marker("rollback-evidence-path"),
            digest_sha256: marker("rollback-evidence-digest"),
            byte_length: 78_102,
            link_count: 78_112,
            kind: InputKind::Symlink,
        };
        self.reviewed_artifacts = vec![BoundInput {
            relative_path: marker("reviewed-artifacts-path"),
            digest_sha256: marker("reviewed-artifacts-digest"),
            byte_length: 78_103,
            link_count: 78_113,
            kind: InputKind::Special,
        }];
        self.binding_sha256 = marker("binding-sha256");
        self.review_id = marker("review-id");
        self.attestation_sha256 = marker("attestation-sha256");
        drop(marker);

        sentinels.extend(
            [
                "78101",
                "78102",
                "78103",
                "78111",
                "78112",
                "78113",
                "Directory",
                "Symlink",
                "Special",
            ]
            .into_iter()
            .map(str::to_owned),
        );
        sentinels
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionStatus {
    ImprovementCandidate,
    Rejected,
}
