use super::super::{
    AdvisoryPractice, AuthorityAnalysis, BindingProductRequirement, BoundInput,
    EvaluationDataControls, EvaluationDataControlsDefinition, EvaluationExecutionBinding,
    EvaluationExecutionBindingRequest, EvaluationLedgerState, EvaluationSpec,
    ExperimentalHypothesis, FactTemporalScope, FileEvaluationExecutionLedger,
    FilePromotionReviewLedger, ImpactAnalysis, InputKind, LawChangeProposal, MigrationAnalysis,
    PromotionLedgerBinding, PromotionLedgerState, PromotionReviewAuthority, ProofAnalysis,
    ProposalAnalyses, RejectedRecommendation, ResearchAudit, ResearchSource, ResearchSourceClass,
    ResearchSourceRecord, ResearchSourceRecordDefinition, VerifiedSourceFact,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[path = "../../../tests/evaluation_runtime_contract/production_native_fixture_boundary_accepts_only_fixed_protected_substrates.rs"]
mod production_native;
use production_native::{
    RESEARCH_2026_01_01_EPOCH, RESEARCH_CHECKED_DAY_EPOCH, RESEARCH_MUTABLE_FRESHNESS_SECONDS,
    research_epoch, research_laws, task,
};
include!("../../../tests/evaluation_runtime_contract/next_root.rs");
include!("../../../tests/evaluation_runtime_contract/research/source/url.rs");
include!("../../../tests/evaluation_runtime_contract/research/proposal.rs");
include!(
    "../../../tests/evaluation_runtime_contract/research/source/record_fields_and_canonical_binding_fail_closed.rs"
);
include!(
    "../../../tests/evaluation_runtime_contract/research/source/substitution_mutate_restore_and_classification_laundering_fail_closed.rs"
);
include!(
    "../../../tests/evaluation_runtime_contract/research/external_stable_fact_and_rebound_laundering_fail_closed.rs"
);
include!(
    "../../../tests/evaluation_runtime_contract/research/production_clock_cannot_be_backdated_or_rebound_by_the_caller.rs"
);
include!(
    "../../../tests/evaluation_runtime_contract/research/every_duplicate_source_and_proposal_participant_is_ineligible.rs"
);
include!(
    "../../../tests/evaluation_runtime_contract/research/read_parse_query_and_audit_paths_are_recursively_zero_write.rs"
);
include!("../../../tests/evaluation_runtime_contract/execution/binding.rs");
include!(
    "../../../tests/evaluation_runtime_contract/execution/final_named_root_revalidation_refuses_orphan_write_and_read_success.rs"
);
include!(
    "../../../tests/evaluation_runtime_contract/execution/and_promotion_roots_reject_symlinked_ancestors.rs"
);
include!(
    "../../../tests/evaluation_runtime_contract/promotion/anchor_journal_recovers_state_only_rollback_and_rejects_paired_restore.rs"
);
include!(
    "../../../tests/evaluation_runtime_contract/promotion/lock_replacement_cannot_create_a_second_mutation_authority.rs"
);
include!(
    "../../../tests/evaluation_runtime_contract/promotion/final_named_state_revalidation_refuses_recoverable_late_swap_success.rs"
);
include!(
    "../../../tests/evaluation_runtime_contract/execution/reservation_has_exactly_one_two_process_winner.rs"
);
include!(
    "../../../tests/evaluation_runtime_contract/evaluation_worker_result_is_exact_typed_and_self_excluded.rs"
);
