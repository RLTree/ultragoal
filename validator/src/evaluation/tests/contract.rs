use super::super::{
    BehaviorOutcome, BoundInput, CapturedTaskObservation, CapturedTaskObservationRecord,
    EvaluationDatasetProvenance, EvaluationError, EvaluationExecutionBinding,
    EvaluationExecutionBindingRequest, EvaluationExecutor, EvaluationRun, EvaluationSpec,
    EvaluationTask, EvaluationTaskDefinition, FailureCase, FilePromotionReviewLedger, InputKind,
    PerturbationControl, PromotionDecision, PromotionEvidencePaths, PromotionLedgerBinding,
    PromotionReview, PromotionReviewAuthority, PromotionReviewEvidence, PromotionStatus, TaskAudit,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

include!("../../../tests/evaluation_contract/next_root.rs");
include!("../../../tests/evaluation_contract/review_authority_teardown.rs");
include!("../../../tests/evaluation_contract/test_review_authority_authority_id.rs");
include!("../../../tests/evaluation_contract/known_training_overlap_is_a_contamination_finding.rs");
include!("../../../tests/evaluation_contract/self_review_cannot_promote.rs");
include!(
    "../../../tests/evaluation_contract/missing_observed_reward_control_blocks_run_not_just_promotion.rs"
);

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

fn private_ledger_root() -> PathBuf {
    let ledger_root = std::env::temp_dir().join(format!(
        "hul-evaluation-contract-review-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::SeqCst),
    ));
    fs::create_dir(&ledger_root).unwrap();
    fs::set_permissions(&ledger_root, fs::Permissions::from_mode(0o700)).unwrap();
    ledger_root
}

fn review_authority(baseline: &EvaluationRun, candidate: &EvaluationRun) -> ReviewAuthorityHarness {
    ReviewAuthorityHarness::current(baseline, candidate)
}
