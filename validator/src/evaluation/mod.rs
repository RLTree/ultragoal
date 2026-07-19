//! Vendor-neutral, candidate-bound evaluation and promotion decisions.
//!
//! Execution is deliberately crate-controlled. External callers may describe
//! evaluation inputs and inspect records, but they cannot implement the sealed
//! executor or mint captured task observations.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

mod ledger;
mod production_input;
mod promotion_ledger;
mod records;
mod research;
pub(crate) mod runtime;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(in crate::evaluation) use ledger::TestFileEvaluationExecutionLedger as FileEvaluationExecutionLedger;
pub(crate) use ledger::{
    EvaluationExecutionBinding, EvaluationExecutionBindingRequest, EvaluationLedgerError,
    EvaluationLedgerState, ExecutionReservationOutcome,
};
pub(crate) use promotion_ledger::{
    FilePromotionReviewLedger, PromotionLedgerBinding, PromotionLedgerState,
};
pub use records::{
    CanonicalEvaluationFailure, CanonicalEvaluationReview, CanonicalEvaluationRun,
    ConfigurationExposure, ConfigurationSource, EvaluationEventKind, PrivacySafeEvaluationEvent,
    PrivacySafeEvaluationEventRecord, RuntimeConfiguration,
};
pub use research::{
    AdvisoryPractice, AuthorityAnalysis, BindingProductRequirement, ExperimentalHypothesis,
    FactTemporalScope, ImpactAnalysis, LawChangeProposal, MigrationAnalysis, ProofAnalysis,
    ProposalAnalyses, RejectedRecommendation, ResearchAudit, ResearchFinding, ResearchSource,
    ResearchSourceClass, ResearchSourceRecord, ResearchSourceRecordDefinition, VerifiedSourceFact,
};
pub(crate) use runtime::{ProductionEvaluationRun, ProductionRuntimeError};

include!("max_tasks.rs");

include!("data_controls.rs");

include!("task.rs");

include!("specification.rs");

include!("captured_task_observation.rs");

include!("local_run.rs");

include!("promotion/display.rs");

include!("promotion/evidence_descriptor.rs");

include!("promotion/authority.rs");

include!("promotion/issuance.rs");

include!("promotion/decision.rs");

include!("promotion/reconciliation.rs");

include!("runs_comparable.rs");

include!("totals.rs");
