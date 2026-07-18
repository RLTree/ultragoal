use super::super::{
    BehaviorOutcome, BoundInput, CapturedTaskObservation, CapturedTaskObservationRecord,
    EvaluationDatasetProvenance, EvaluationError, EvaluationExecutor, EvaluationRun,
    EvaluationSpec, EvaluationTask, EvaluationTaskDefinition, FailureCase, InputKind,
    PerturbationControl, PromotionDecision, PromotionReview, PromotionReviewAuthority,
    PromotionReviewEvidence, PromotionStatus, TaskAudit,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

include!("../../../../tests/evaluation_contract/next_root.rs");
include!("../../../../tests/evaluation_contract/test_review_authority_authority_id.rs");
include!(
    "../../../../tests/evaluation_contract/known_training_overlap_is_a_contamination_finding.rs"
);
include!("../../../../tests/evaluation_contract/self_review_cannot_promote.rs");
include!(
    "../../../../tests/evaluation_contract/missing_observed_reward_control_blocks_run_not_just_promotion.rs"
);
