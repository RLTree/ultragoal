pub(crate) struct PromotionEvidencePaths {
    pub(crate) representative_journey: String,
    pub(crate) rollback_evidence: String,
    pub(crate) reviewed_artifacts: Vec<String>,
}

struct RetainedPromotionEvidence {
    root_path: std::path::PathBuf,
    root: std::fs::File,
    root_identity: ledger::FileIdentity,
    inputs: Vec<RetainedPromotionInput>,
}

struct RetainedPromotionInput {
    bound: BoundInput,
    identity: ledger::FileIdentity,
    descriptor: std::fs::File,
}

/// A private, linear issuer/consumer. Its only constructor binds a durable
/// promotion ledger to descriptors retained for the whole review lifecycle.
pub(crate) struct PromotionReviewAuthority {
    ledger: FilePromotionReviewLedger,
    evidence: Option<RetainedPromotionEvidence>,
    issuance_available: bool,
}

impl FilePromotionReviewLedger {
    pub(crate) fn bind_review_evidence(
        self,
        root_path: impl AsRef<std::path::Path>,
    ) -> Result<PromotionReviewAuthority, EvaluationError> {
        let root_path = root_path.as_ref().to_path_buf();
        let (root, root_identity) = ledger::open_safe_directory(&root_path)
            .map_err(|_| EvaluationError::new("evaluation-review-evidence-root-unsafe"))?;
        Ok(PromotionReviewAuthority {
            ledger: self,
            evidence: Some(RetainedPromotionEvidence {
                root_path,
                root,
                root_identity,
                inputs: Vec::new(),
            }),
            issuance_available: true,
        })
    }
}

impl PromotionReviewAuthority {
    pub(crate) fn issue_review(
        &mut self,
        baseline: &EvaluationRun,
        candidate: &EvaluationRun,
        paths: PromotionEvidencePaths,
    ) -> Result<PromotionReview, EvaluationError> {
        if !self.issuance_available {
            return Err(EvaluationError::new(
                "evaluation-review-duplicate-issuer-refused",
            ));
        }
        let evidence = self
            .evidence
            .as_mut()
            .ok_or_else(|| EvaluationError::new("evaluation-review-issuer-unbound"))?;
        if !evidence.inputs.is_empty() || paths.reviewed_artifacts.is_empty() {
            return Err(EvaluationError::new(
                "evaluation-review-duplicate-issuer-refused",
            ));
        }
        let mut names = paths.reviewed_artifacts;
        names.sort();
        names.insert(0, paths.rollback_evidence);
        names.insert(0, paths.representative_journey);
        evidence.inputs = names
            .iter()
            .map(|path| capture_promotion_input(&evidence.root, path))
            .collect::<Result<_, _>>()?;
        evidence.revalidate()?;
        let review = PromotionReview::issue(
            baseline,
            candidate,
            PromotionReviewEvidence {
                representative_journey: evidence.inputs[0].bound.clone(),
                rollback_evidence: evidence.inputs[1].bound.clone(),
                reviewed_artifacts: evidence.inputs[2..]
                    .iter()
                    .map(|input| input.bound.clone())
                    .collect(),
            },
            self,
        )?;
        if !self.retained_evidence_is_current(&review) {
            return Err(EvaluationError::new("evaluation-review-evidence-changed"));
        }
        self.issuance_available = false;
        Ok(review)
    }

    pub(super) fn authority_id(&self) -> &str {
        self.ledger.authority_id()
    }
    pub(super) fn reviewer_id(&self) -> &str {
        self.ledger.reviewer_id()
    }
    pub(super) fn review_session_id(&self) -> &str {
        self.ledger.review_session_id()
    }
    pub(super) fn current_binding(&self) -> (&str, &str) {
        self.ledger.current_review_binding()
    }

    fn issue_attestation(&mut self, binding_sha256: &str) -> Result<String, EvaluationError> {
        self.evidence
            .as_ref()
            .ok_or_else(|| EvaluationError::new("evaluation-review-issuer-unbound"))?
            .revalidate()?;
        self.ledger
            .issue_bound_attestation(binding_sha256)
            .map_err(|_| EvaluationError::new("evaluation-review-ledger-issuance-refused"))
    }

    pub(super) fn verify_and_consume(
        &mut self,
        binding_sha256: &str,
        reviewer_id: &str,
        review_id: &str,
        attestation_sha256: &str,
    ) -> bool {
        let current = self
            .evidence
            .as_ref()
            .is_some_and(|evidence| evidence.revalidate().is_ok());
        current
            && matches!(
                self.ledger.consume_attestation(
                    binding_sha256,
                    reviewer_id,
                    review_id,
                    attestation_sha256
                ),
                Ok(promotion_ledger::PromotionConsumptionOutcome::Consumed)
            )
            && self
                .evidence
                .as_ref()
                .is_some_and(|evidence| evidence.revalidate().is_ok())
    }

    fn retained_evidence_is_current(&self, review: &PromotionReview) -> bool {
        self.evidence.as_ref().is_some_and(|evidence| {
            evidence.revalidate().is_ok()
                && evidence.inputs.len() == review.reviewed_artifacts.len() + 2
                && evidence.inputs[0].bound == review.representative_journey
                && evidence.inputs[1].bound == review.rollback_evidence
                && evidence.inputs[2..]
                    .iter()
                    .map(|input| &input.bound)
                    .eq(review.reviewed_artifacts.iter())
        })
    }
}

impl RetainedPromotionEvidence {
    fn revalidate(&self) -> Result<(), EvaluationError> {
        self.require_named_root()?;
        for input in &self.inputs {
            let named = open_promotion_input(&self.root, input.bound.relative_path())?;
            if ledger::safe_file_identity(&named).ok() != Some(input.identity)
                || ledger::safe_file_identity(&input.descriptor).ok() != Some(input.identity)
                || read_promotion_input(&input.descriptor, input.identity.length)?
                    != input.bound.digest_sha256()
            {
                return Err(EvaluationError::new("evaluation-review-evidence-changed"));
            }
        }
        self.require_named_root()
    }

    fn require_named_root(&self) -> Result<(), EvaluationError> {
        let (named, identity) = ledger::open_safe_directory(&self.root_path)
            .map_err(|_| EvaluationError::new("evaluation-review-evidence-root-changed"))?;
        if identity != self.root_identity
            || ledger::safe_file_identity(&self.root).ok() != Some(self.root_identity)
        {
            return Err(EvaluationError::new(
                "evaluation-review-evidence-root-changed",
            ));
        }
        drop(named);
        Ok(())
    }
}

fn capture_promotion_input(
    root: &std::fs::File,
    path: &str,
) -> Result<RetainedPromotionInput, EvaluationError> {
    let descriptor = open_promotion_input(root, path)?;
    let identity = ledger::safe_file_identity(&descriptor)
        .map_err(|_| EvaluationError::new("evaluation-review-evidence-stat-failed"))?;
    if identity.mode & libc::S_IFMT as u32 != libc::S_IFREG as u32
        || identity.links != 1
        || identity.length == 0
        || identity.length > MAX_INPUT_BYTES
    {
        return Err(EvaluationError::new("evaluation-review-evidence-unsafe"));
    }
    let digest_sha256 = read_promotion_input(&descriptor, identity.length)?;
    Ok(RetainedPromotionInput {
        bound: BoundInput::regular(path, digest_sha256, identity.length),
        identity,
        descriptor,
    })
}
