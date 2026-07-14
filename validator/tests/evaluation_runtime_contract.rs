#[path = "../src/digest.rs"]
mod digest;
#[path = "../src/evaluation/mod.rs"]
mod evaluation;
#[path = "../src/fixture_scheduler/mod.rs"]
mod fixture_scheduler;

// Recreate the production-only capture dependency closure in this integration
// crate. The repository's legacy capture harness intentionally omits the
// scheduler adapter under cfg(test), so this focused contract imports the real
// adapter directly and supplies only its two-stream bounded-output test seam.
mod environment {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(super) enum InvocationSensitivity {
        Public,
        SecretBearing,
    }

    impl InvocationSensitivity {
        pub(super) fn from_bound_secrets(secrets: &[Vec<u8>]) -> Self {
            if secrets.iter().any(|secret| !secret.is_empty()) {
                Self::SecretBearing
            } else {
                Self::Public
            }
        }

        pub(super) const fn is_secret_bearing(self) -> bool {
            matches!(self, Self::SecretBearing)
        }
    }
}
mod output {
    use super::environment::InvocationSensitivity;
    use std::io::Read;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    pub(super) struct OutputBudget {
        limit: u64,
        observed: AtomicU64,
        exceeded: AtomicBool,
    }

    pub(super) struct PendingOutput(Vec<u8>);

    pub(super) struct CapturedOutput(Vec<u8>);

    impl CapturedOutput {
        pub(super) fn retained(&self) -> &[u8] {
            &self.0
        }
    }

    pub(super) struct StableOutputs {
        pub(super) first: CapturedOutput,
        pub(super) second: CapturedOutput,
        pub(super) output_limit_exceeded: bool,
    }

    impl OutputBudget {
        pub(super) fn for_sensitivity(limit: usize, _sensitivity: InvocationSensitivity) -> Self {
            Self {
                limit: limit as u64,
                observed: AtomicU64::new(0),
                exceeded: AtomicBool::new(false),
            }
        }

        fn claim(&self, requested: usize) -> usize {
            loop {
                let observed = self.observed.load(Ordering::SeqCst);
                let remaining = self.limit.saturating_sub(observed);
                let claimed = remaining.min(requested as u64);
                if self
                    .observed
                    .compare_exchange(
                        observed,
                        observed + claimed,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    )
                    .is_ok()
                {
                    if claimed < requested as u64 || claimed == 0 {
                        self.exceeded.store(true, Ordering::SeqCst);
                    }
                    return claimed as usize;
                }
            }
        }

        pub(super) fn exceeded(&self) -> bool {
            self.exceeded.load(Ordering::SeqCst)
        }

        pub(super) fn finalize_streams(
            &self,
            first: PendingOutput,
            second: PendingOutput,
        ) -> StableOutputs {
            StableOutputs {
                first: CapturedOutput(first.0),
                second: CapturedOutput(second.0),
                output_limit_exceeded: self.exceeded(),
            }
        }
    }

    pub(super) fn observe(
        mut reader: impl Read,
        _limit: usize,
        budget: &OutputBudget,
    ) -> Result<PendingOutput, String> {
        let mut retained = Vec::new();
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|_| "captured output stream read failed".to_owned())?;
            if read == 0 {
                break;
            }
            let allowed = budget.claim(read);
            retained.extend_from_slice(&buffer[..allowed]);
            if allowed < read {
                break;
            }
        }
        Ok(PendingOutput(retained))
    }
}
#[path = "../src/cli/capture/fixture/mod.rs"]
mod fixture_capture;

use evaluation::runtime::{
    FixtureEvaluationBridge, FixtureTaskRequest, ProductionRuntimeError, execute_production,
};
use evaluation::{
    AdvisoryPractice, AuthorityAnalysis, BindingProductRequirement, BoundInput,
    ConfigurationExposure, EvaluationDataControls, EvaluationDataControlsDefinition,
    EvaluationExecutionBinding, EvaluationExecutionBindingRequest, EvaluationLedgerState,
    EvaluationSpec, EvaluationTask, EvaluationTaskDefinition, ExperimentalHypothesis,
    FactTemporalScope, FileEvaluationExecutionLedger, FilePromotionReviewLedger, ImpactAnalysis,
    InputKind, LawChangeProposal, MigrationAnalysis, PerturbationControl, PromotionLedgerBinding,
    PromotionLedgerState, PromotionReviewAuthority, ProofAnalysis, ProposalAnalyses,
    RejectedRecommendation, ResearchAudit, ResearchSource, ResearchSourceClass,
    ResearchSourceRecord, ResearchSourceRecordDefinition, RuntimeConfiguration, VerifiedSourceFact,
};
use fixture_capture::FixtureCaptureAdapter;
use fixture_scheduler::{
    ConfinementPolicy, ExpectedOutcome, FixtureExecutionRecord, FixtureExecutionRecordCapture,
    FixtureKind, FixtureScheduler, FixtureSpec, NetworkIsolation, ObservedOutcome, ResourceKind,
    RunDisposition,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

include!("evaluation_runtime_contract/next_root.rs");

include!(
    "evaluation_runtime_contract/production_native_fixture_boundary_accepts_only_fixed_protected_substrates.rs"
);

include!("evaluation_runtime_contract/research_source_url.rs");

include!("evaluation_runtime_contract/research_proposal.rs");

include!(
    "evaluation_runtime_contract/research_source_record_fields_and_canonical_binding_fail_closed.rs"
);

include!(
    "evaluation_runtime_contract/research_source_substitution_mutate_restore_and_classification_laundering_fail_closed.rs"
);

include!(
    "evaluation_runtime_contract/research_external_stable_fact_and_rebound_laundering_fail_closed.rs"
);

include!(
    "evaluation_runtime_contract/research_production_clock_cannot_be_backdated_or_rebound_by_the_caller.rs"
);

include!(
    "evaluation_runtime_contract/research_every_duplicate_source_and_proposal_participant_is_ineligible.rs"
);

include!(
    "evaluation_runtime_contract/research_read_parse_query_and_audit_paths_are_recursively_zero_write.rs"
);

include!("evaluation_runtime_contract/execution_binding.rs");

include!(
    "evaluation_runtime_contract/execution_final_named_root_revalidation_refuses_orphan_write_and_read_success.rs"
);

include!("evaluation_runtime_contract/execution_and_promotion_roots_reject_symlinked_ancestors.rs");

include!(
    "evaluation_runtime_contract/promotion_anchor_journal_recovers_state_only_rollback_and_rejects_paired_restore.rs"
);

include!(
    "evaluation_runtime_contract/promotion_lock_replacement_cannot_create_a_second_mutation_authority.rs"
);

include!(
    "evaluation_runtime_contract/promotion_final_named_state_revalidation_refuses_recoverable_late_swap_success.rs"
);

include!("evaluation_runtime_contract/execution_reservation_has_exactly_one_two_process_winner.rs");

include!(
    "evaluation_runtime_contract/evaluation_worker_result_is_exact_typed_and_self_excluded.rs"
);
