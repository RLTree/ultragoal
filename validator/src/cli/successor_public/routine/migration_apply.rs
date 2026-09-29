use super::host::RoutineStateQuarantinePlan;
use super::*;
use crate::cli::successor::command_contract::MigrateAction;
use crate::cli::successor::runtime::{Diagnostic, DiagnosticDetails, DiagnosticId};
use crate::cli::successor::{ExitClass, OptionName};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

const PLAN_LIMIT: u64 = 1024 * 1024;
const ADMISSION_SCHEMA: &str = "RoutineStateMigrationAdmission-v7";
const PLAN_SCHEMA: &str = "RoutineStateQuarantinePlan-v5";
const SUPPORT_LIMIT: &str = "read-only legacy HostState admission plus exact accepted no-gap whole-owner quarantine; a non-current public head is not historical-head proof, a verified fresh v8 stage atomically replaces the legacy owner before that unchanged legacy tree is finalized under a deterministic sibling, permanent exact pending and settled receipts bind the terminal migrated layout, and no legacy authority is imported; no routine execution, ProductState, Product Fitness, readiness, or release claim";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RoutineStateMigrationRecord {
    schema_version: String,
    status: String,
    format_status: String,
    legacy_singleton_count: usize,
    canonical_continuation_count: usize,
    event_journal_count: usize,
    history_relation: String,
    migration_effect: String,
    migration_authorized: bool,
    reserved_recovery: Option<RoutineReservedRecoveryRecord>,
    quarantine_plan: Option<RoutineStateQuarantinePlan>,
    next_action: String,
    claim_effect: String,
    support_limit: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RoutineReservedRecoveryRecord {
    schema_version: String,
    status: String,
    checkpoint_relation: String,
    checkpoint_head_evidence: String,
    private_attempt_state: String,
    effect_evidence: String,
    owner_observation: String,
    recovery_effect: String,
    recovery_authorized: bool,
    claim_effect: String,
}

#[derive(Serialize)]
struct RoutineStateQuarantineApplyRecord<'a> {
    schema_version: &'static str,
    status: &'a str,
    effect: &'a str,
    settlement_state: &'a str,
    plan_id: &'a str,
    quarantine_owner: &'a str,
    fresh_format_verified: bool,
    accepted_plan_consumed: bool,
    legacy_authority_imported: bool,
    claim_effect: &'static str,
    support_limit: &'static str,
}

#[derive(Serialize)]
struct RoutineStateAbandonmentApplyRecord<'a> {
    schema_version: &'static str,
    status: &'a str,
    effect: &'a str,
    settlement_state: &'a str,
    plan_id: &'a str,
    quarantine_owner: &'a str,
    fresh_format_verified: bool,
    accepted_plan_consumed: bool,
    retirement_approved: bool,
    legacy_history_continuity: &'static str,
    legacy_authority_imported: bool,
    previous_reuse_and_recovery: &'static str,
    possible_duplicate_effect: &'static str,
    claim_effect: &'static str,
    support_limit: &'static str,
}

enum AcceptedQuarantineRecord {
    Migration(RoutineStateMigrationRecord),
    Abandonment(RoutineStateAbandonmentRecord),
}

impl AcceptedQuarantineRecord {
    fn plan(&self) -> &RoutineStateQuarantinePlan {
        match self {
            Self::Migration(record) => record.quarantine_plan.as_ref().unwrap(),
            Self::Abandonment(record) => &record.quarantine_plan,
        }
    }

    fn matches(&self, admission: &RoutineStateMigrationAdmission) -> bool {
        match self {
            Self::Migration(record) => migration_admission_record(admission) == *record,
            Self::Abandonment(record) => abandonment_record(admission).as_ref() == Some(record),
        }
    }

    const fn is_abandonment(&self) -> bool {
        matches!(self, Self::Abandonment(_))
    }
}

struct ApplyArguments<'a> {
    requested_target: Option<&'a str>,
    relative_plan: &'a str,
    accepted_plan: &'a str,
    retirement_approved: bool,
}

pub(crate) fn migration_admission_record(
    admission: &RoutineStateMigrationAdmission,
) -> RoutineStateMigrationRecord {
    RoutineStateMigrationRecord {
        schema_version: ADMISSION_SCHEMA.to_owned(),
        status: admission.status.to_owned(),
        format_status: admission.format_status.to_owned(),
        legacy_singleton_count: admission.legacy_singleton_count,
        canonical_continuation_count: admission.canonical_continuation_count,
        event_journal_count: admission.event_journal_count,
        history_relation: admission.history_relation.to_owned(),
        migration_effect: "none".to_owned(),
        migration_authorized: false,
        reserved_recovery: admission.reserved_recovery.as_ref().map(|assessment| {
            RoutineReservedRecoveryRecord {
                schema_version: "RoutineReservedRecoveryAssessment-v1".to_owned(),
                status: assessment.status.to_owned(),
                checkpoint_relation: "noncurrent_head_matching_private_attempt".to_owned(),
                checkpoint_head_evidence: "noncurrent_value_only_historical_head_unproven"
                    .to_owned(),
                private_attempt_state: "reserved".to_owned(),
                effect_evidence: assessment.effect_evidence.to_owned(),
                owner_observation: assessment.owner_observation.to_owned(),
                recovery_effect: "none".to_owned(),
                recovery_authorized: false,
                claim_effect: "none".to_owned(),
            }
        }),
        quarantine_plan: admission.quarantine_plan.clone(),
        next_action: admission.next_action.to_owned(),
        claim_effect: "none".to_owned(),
        support_limit: SUPPORT_LIMIT.to_owned(),
    }
}

pub(crate) fn apply_state_quarantine(
    root: &Path,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> RuntimeOutcome {
    let Some(arguments) = apply_arguments(invocation) else {
        return failure(
            ExitClass::InvalidInvocation,
            DiagnosticId::UnexpectedArguments,
            "migrate apply requires one relative --plan, its exact --accept-plan identity, and retirement approval only for an abandonment record",
            "reparse the exact migrate apply command through the successor grammar",
            "none",
        );
    };
    let Some(home) = home else {
        return failure(
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "personal HostState authority is unavailable",
            "run with one explicit personal home authority",
            "none",
        );
    };
    let root = match root.canonicalize() {
        Ok(root) => root,
        Err(_) => {
            return failure(
                ExitClass::ActionableFinding,
                DiagnosticId::ContextUnavailable,
                "the repository root could not be resolved to one canonical directory",
                "retry from the exact repository root",
                "none",
            );
        }
    };
    let plan_path = root.join(arguments.relative_plan);
    let bytes = match super::super::fit::external_plan_file::read_immutable_plan(
        &plan_path, PLAN_LIMIT,
    ) {
        Ok(bytes) => bytes,
        Err(_) => {
            return failure(
                ExitClass::BlockedAuthority,
                DiagnosticId::AuthorityRequired,
                "the accepted quarantine record is not one immutable confined input",
                "write the exact diagnose or abandon-plan JSON to the declared in-repository plan path without aliases or hard links",
                "none",
            );
        }
    };
    let Some(canonical) = exact_json_frame(&bytes) else {
        return failure(
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "the accepted quarantine record is not exact canonical closed JSON",
            "use the unmodified JSON bytes emitted by ultragoal --json diagnose or migrate abandon-plan",
            "none",
        );
    };
    let migration = exact_closed_record::<RoutineStateMigrationRecord>(canonical);
    let abandonment = exact_closed_record::<RoutineStateAbandonmentRecord>(canonical);
    let accepted = match (migration, abandonment, arguments.retirement_approved) {
        (Some(record), None, false)
            if validate_accepted_record(&record, arguments.accepted_plan).is_some() =>
        {
            AcceptedQuarantineRecord::Migration(record)
        }
        (None, Some(record), true)
            if validate_abandonment_record(&record, arguments.accepted_plan).is_some() =>
        {
            AcceptedQuarantineRecord::Abandonment(record)
        }
        _ => {
            return failure(
                ExitClass::BlockedAuthority,
                DiagnosticId::AuthorityRequired,
                "the accepted record, plan identity, or abandonment approval was substituted",
                "repeat diagnose or abandon-plan, explicitly accept that exact plan identity, and include --approve-retirement only for the abandonment record",
                "none",
            );
        }
    };
    let plan = accepted.plan();
    let context = match super::super::current_external_context(&root) {
        Ok(context) => context,
        Err(()) => {
            return failure(
                ExitClass::ActionableFinding,
                DiagnosticId::ContextUnavailable,
                "a current external-write context could not be constructed",
                "stabilize the repository and retry from its canonical root",
                "none",
            );
        }
    };
    let binding = match current_diagnosis_binding(&root, arguments.requested_target, &context) {
        Ok(Some(binding)) => binding,
        _ => {
            return failure(
                ExitClass::BlockedAuthority,
                DiagnosticId::AuthorityRequired,
                "the current routine diagnosis binding is unavailable",
                "restore the exact routine manifest and source binding before retrying",
                "none",
            );
        }
    };
    let execution_id = match checkpoint_execution_id(&binding) {
        Ok(execution_id) => execution_id,
        Err(_) => {
            return failure(
                ExitClass::ActionableFinding,
                DiagnosticId::StateUnavailable,
                "the current routine checkpoint binding could not be derived",
                "stabilize the candidate and recompute diagnose before retrying",
                "none",
            );
        }
    };
    let checkpoint_binding = host::CheckpointBinding::new(
        binding.target(),
        binding.context().context_id(),
        binding.plan().binding().candidate_id(),
        binding.plan().plan_id(),
        binding.snapshot().snapshot_id(),
        &execution_id,
    );
    let current_assessment = if accepted.is_abandonment() {
        HostState::assess_abandonment_admission(home, checkpoint_binding)
    } else {
        HostState::assess_migration_admission(home, checkpoint_binding)
    };
    let current = match current_assessment {
        Ok(Some(current)) => current,
        Ok(None) | Err(_) => {
            return failure(
                ExitClass::BlockedAuthority,
                DiagnosticId::AuthorityRequired,
                "the current HostState quarantine admission is unavailable or unsafe",
                "preserve HostState and recompute the exact diagnose or abandonment record",
                "none",
            );
        }
    };
    let exact_replay = current.status == "already_current"
        && current.format_status == "routine_host_state_v8"
        && current.reserved_recovery.is_none()
        && current.quarantine_plan.is_none();
    let exact_transition = current.status == "quarantine_transition_preserve_and_hold"
        && current.format_status == "quarantine_or_bootstrap_transition"
        && current.history_relation == "transition_requires_exact_accepted_plan"
        && current.reserved_recovery.is_none()
        && current.quarantine_plan.is_none();
    if (!exact_replay && !exact_transition && !accepted.matches(&current))
        || context.revalidate().is_err()
        || binding.context().revalidate().is_err()
    {
        return failure(
            ExitClass::ActionableFinding,
            DiagnosticId::StaleContext,
            "the accepted quarantine record is no longer byte-equivalent to current evidence",
            "discard the stale record and explicitly accept one freshly derived plan",
            "none",
        );
    }
    let outcome = match if accepted.is_abandonment() {
        HostState::apply_abandonment_plan(
            home,
            binding.target(),
            checkpoint_binding,
            plan,
            |admission| accepted.matches(admission),
        )
    } else {
        HostState::apply_quarantine_plan(
            home,
            binding.target(),
            checkpoint_binding,
            plan,
            |admission| accepted.matches(admission),
        )
    } {
        Ok(outcome) => outcome,
        Err(error) => return outcome::failure(PublicFailure::Host(error)),
    };
    let contexts_current = context.revalidate().is_ok() && binding.context().revalidate().is_ok();
    project_apply_outcome(outcome, contexts_current, accepted.is_abandonment())
}

fn project_apply_outcome(
    outcome: super::host::RoutineStateQuarantineApplyOutcome,
    contexts_current: bool,
    abandonment: bool,
) -> RuntimeOutcome {
    if !contexts_current {
        return failure(
            ExitClass::ActionableFinding,
            DiagnosticId::StaleContext,
            "the repository candidate changed while HostState migration was settling",
            "preserve the quarantine and fresh owner, then diagnose before any routine effect",
            outcome.effect,
        );
    }
    if abandonment {
        return project_abandonment_apply_outcome(outcome);
    }
    let record = RoutineStateQuarantineApplyRecord {
        schema_version: "RoutineStateQuarantineApplyOutcome-v3",
        status: outcome.status,
        effect: outcome.effect,
        settlement_state: outcome.settlement_state,
        plan_id: &outcome.plan_id,
        quarantine_owner: &outcome.quarantine_owner,
        fresh_format_verified: outcome.fresh_format_verified,
        accepted_plan_consumed: true,
        legacy_authority_imported: false,
        claim_effect: "host_state_migration_only",
        support_limit: SUPPORT_LIMIT,
    };
    match serde_json::to_vec(&record) {
        Ok(machine) if super::super::public_output_allowed(machine.len()) => {
            RuntimeOutcome::payload(
                if matches!(
                    outcome.status,
                    "ambiguous_hold"
                        | "settlement_receipt_observed_durability_unacknowledged"
                        | "settlement_receipt_durable_post_verify_hold"
                ) {
                    ExitClass::ActionableFinding
                } else {
                    ExitClass::Success
                },
                machine,
                format!(
                    "routine state quarantine status={} effect={}",
                    outcome.status, outcome.effect
                ),
            )
        }
        _ => failure(
            ExitClass::InternalFailure,
            DiagnosticId::ProjectionFailed,
            "the bounded HostState apply outcome could not be encoded",
            "preserve the quarantine and diagnose before retrying",
            outcome.effect,
        ),
    }
}

fn project_abandonment_apply_outcome(
    outcome: super::host::RoutineStateQuarantineApplyOutcome,
) -> RuntimeOutcome {
    let record = RoutineStateAbandonmentApplyRecord {
        schema_version: "RoutineStateAbandonmentApplyOutcome-v1",
        status: outcome.status,
        effect: outcome.effect,
        settlement_state: outcome.settlement_state,
        plan_id: &outcome.plan_id,
        quarantine_owner: &outcome.quarantine_owner,
        fresh_format_verified: outcome.fresh_format_verified,
        accepted_plan_consumed: true,
        retirement_approved: true,
        legacy_history_continuity: "abandoned_not_imported",
        legacy_authority_imported: false,
        previous_reuse_and_recovery: "unavailable",
        possible_duplicate_effect: "declared_target_local_routine_outputs_may_reexecute",
        claim_effect: "host_state_abandonment_only",
        support_limit: ABANDONMENT_SUPPORT_LIMIT,
    };
    match serde_json::to_vec(&record) {
        Ok(machine) if super::super::public_output_allowed(machine.len()) => {
            RuntimeOutcome::payload(
                apply_exit_class(outcome.status),
                machine,
                format!(
                    "routine legacy history abandonment status={} effect={}",
                    outcome.status, outcome.effect
                ),
            )
        }
        _ => failure(
            ExitClass::InternalFailure,
            DiagnosticId::ProjectionFailed,
            "the bounded HostState abandonment outcome could not be encoded",
            "preserve the quarantine and diagnose before retrying",
            outcome.effect,
        ),
    }
}

fn apply_exit_class(status: &str) -> ExitClass {
    if matches!(
        status,
        "ambiguous_hold"
            | "settlement_receipt_observed_durability_unacknowledged"
            | "settlement_receipt_durable_post_verify_hold"
    ) {
        ExitClass::ActionableFinding
    } else {
        ExitClass::Success
    }
}

fn apply_arguments(invocation: &ParsedInvocation) -> Option<ApplyArguments<'_>> {
    if invocation.command != SuccessorCommand::Migrate(MigrateAction::Apply)
        || invocation.effect != EffectClass::ExternalWrite
        || !(2..=4).contains(&invocation.arguments.len())
    {
        return None;
    }
    let plan = invocation
        .arguments
        .iter()
        .find(|argument| argument.name == OptionName::Plan)?;
    let accepted = invocation
        .arguments
        .iter()
        .find(|argument| argument.name == OptionName::AcceptPlan)?;
    let retirement = invocation
        .arguments
        .iter()
        .find(|argument| argument.name == OptionName::ApproveRetirement);
    let target = invocation
        .arguments
        .iter()
        .find(|argument| argument.name == OptionName::Target);
    match (&plan.value, &accepted.value) {
        (ParsedValue::RelativePath(plan), ParsedValue::Identifier(accepted)) => {
            let retirement_approved = match retirement {
                Some(argument) if matches!(argument.value, ParsedValue::Flag) => true,
                None => false,
                _ => return None,
            };
            let requested_target = match target {
                Some(argument) => match &argument.value {
                    ParsedValue::RepositoryTarget(target) => Some(target.as_str()),
                    _ => return None,
                },
                None => None,
            };
            Some(ApplyArguments {
                requested_target,
                relative_plan: plan.as_str(),
                accepted_plan: accepted.as_str(),
                retirement_approved,
            })
        }
        _ => None,
    }
}

fn exact_closed_record<T>(bytes: &[u8]) -> Option<T>
where
    T: DeserializeOwned + Serialize,
{
    let record = serde_json::from_slice::<T>(bytes).ok()?;
    (serde_json::to_vec(&record).ok().as_deref() == Some(bytes)).then_some(record)
}

fn exact_json_frame(bytes: &[u8]) -> Option<&[u8]> {
    match bytes.strip_suffix(b"\n") {
        Some(frame) if !frame.ends_with(b"\n") => Some(frame),
        None => Some(bytes),
        Some(_) => None,
    }
}

fn validate_accepted_record<'a>(
    record: &'a RoutineStateMigrationRecord,
    accepted_plan: &str,
) -> Option<&'a RoutineStateQuarantinePlan> {
    let plan = record.quarantine_plan.as_ref()?;
    let normal_candidate = matches!(
        record.status.as_str(),
        "migration_candidate_requires_approval" | "history_relation_established_preserve_and_hold"
    ) && record.reserved_recovery.is_none();
    let stale_reserved_candidate = record.status
        == "stale_reserved_abandoned_candidate_preserve_and_hold"
        && record.history_relation == "redundant_equivalent"
        && plan.history_relation == "redundant_equivalent"
        && plan.authoritative_history == "none_noncurrent_value_only_historical_head_unproven"
        && record.reserved_recovery.as_ref().is_some_and(|reserved| {
            reserved.schema_version == "RoutineReservedRecoveryAssessment-v1"
                && reserved.status == "stale_reserved_abandoned_candidate_preserve_and_hold"
                && reserved.checkpoint_relation == "noncurrent_head_matching_private_attempt"
                && reserved.checkpoint_head_evidence
                    == "noncurrent_value_only_historical_head_unproven"
                && reserved.private_attempt_state == "reserved"
                && reserved.effect_evidence == "pristine_no_effect_observed"
                && reserved.owner_observation == "owner_process_not_observed"
                && reserved.recovery_effect == "none"
                && !reserved.recovery_authorized
                && reserved.claim_effect == "none"
        });
    (record.schema_version == ADMISSION_SCHEMA
        && record.format_status == "absent_legacy"
        && (normal_candidate || stale_reserved_candidate)
        && record.migration_effect == "none"
        && !record.migration_authorized
        && record.claim_effect == "none"
        && record.support_limit == SUPPORT_LIMIT
        && plan.schema_version == PLAN_SCHEMA
        && plan.plan_id == accepted_plan
        && plan.apply_capability == "available_exact_record_only")
        .then_some(plan)
}

fn validate_abandonment_record<'a>(
    record: &'a RoutineStateAbandonmentRecord,
    accepted_plan: &str,
) -> Option<&'a RoutineStateQuarantinePlan> {
    let plan = &record.quarantine_plan;
    let supported_relation = matches!(
        record.history_relation.as_str(),
        "terminal_event_history_incomplete"
            | "terminal_event_history_conflicting"
            | "conflicting_histories_hold"
    );
    (record.schema_version == ABANDONMENT_RECORD_SCHEMA
        && record.status == "legacy_history_abandonment_requires_explicit_retirement_approval"
        && record.format_status == "absent_legacy"
        && supported_relation
        && record.abandonment_effect == "none"
        && !record.abandonment_authorized
        && record.consequences.legacy_history_continuity == "abandoned"
        && !record.consequences.legacy_authority_imported
        && record.consequences.previous_reuse_and_recovery == "unavailable"
        && record.consequences.possible_duplicate_effect
            == "declared_target_local_routine_outputs_may_reexecute"
        && record.claim_effect == "none"
        && record.support_limit == ABANDONMENT_SUPPORT_LIMIT
        && plan.schema_version == ABANDONMENT_PLAN_SCHEMA
        && plan.plan_id == accepted_plan
        && plan.history_relation == record.history_relation
        && plan.authoritative_history == "none_explicit_operator_abandonment"
        && plan.apply_capability == "available_exact_record_only")
        .then_some(plan)
}

fn failure(
    class: ExitClass,
    id: DiagnosticId,
    cause: &'static str,
    repair: &'static str,
    effect: &'static str,
) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        class,
        Diagnostic::new(
            id,
            class,
            DiagnosticDetails {
                cause,
                affected_surface: "routine HostState quarantine apply",
                repair,
                effect,
                rerun: "ultragoal --json migrate apply --plan <relative-diagnosis-record> --accept-plan <routine-quarantine-sha256>",
                ceiling: SUPPORT_LIMIT,
            },
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> RoutineStateQuarantinePlan {
        RoutineStateQuarantinePlan {
            schema_version: PLAN_SCHEMA.to_owned(),
            plan_id: format!("routine-quarantine-sha256:{}", "a".repeat(64)),
            source_inventory_sha256: format!("sha256:{}", "b".repeat(64)),
            source_owner_tree_sha256: format!("sha256:{}", "c".repeat(64)),
            target_path_sha256: format!("sha256:{}", "d".repeat(64)),
            history_relation: "single_history_only".to_owned(),
            authoritative_history: "the_only_authenticated_history".to_owned(),
            source_owner: "routine-public".to_owned(),
            quarantine_owner: format!("routine-public.quarantine-{}", "a".repeat(64)),
            target_format: "routine-host-state-v8".to_owned(),
            strategy: "stage_fresh_v8_then_atomic_exchange_finalize_legacy_and_publish_settlement_receipt"
                .to_owned(),
            apply_capability: "available_exact_record_only".to_owned(),
            operations: vec![
                "acquire_parent_migration_lock_and_legacy_adapter_lock_then_revalidate_exact_source_inventory".to_owned(),
            ],
            rollback: "before_swap_only".to_owned(),
        }
    }

    fn admission() -> RoutineStateMigrationAdmission {
        RoutineStateMigrationAdmission {
            status: "migration_candidate_requires_approval",
            format_status: "absent_legacy",
            legacy_singleton_count: 0,
            canonical_continuation_count: 0,
            event_journal_count: 0,
            history_relation: "single_history_only",
            next_action: "independently review the deterministic reversible quarantine plan before any host write",
            reserved_recovery: None,
            quarantine_plan: Some(plan()),
        }
    }

    #[test]
    fn admission_v7_serializes_one_closed_apply_capable_plan_v5() {
        let record = migration_admission_record(&admission());
        let bytes = serde_json::to_vec(&record).unwrap();
        let decoded: RoutineStateMigrationRecord = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, record);
        assert_eq!(record.schema_version, ADMISSION_SCHEMA);
        let plan = record.quarantine_plan.as_ref().unwrap();
        assert_eq!(plan.schema_version, PLAN_SCHEMA);
        assert_eq!(plan.apply_capability, "available_exact_record_only");
        assert_eq!(validate_accepted_record(&record, &plan.plan_id), Some(plan));
    }

    #[test]
    fn typed_event_history_holds_are_never_apply_capable_without_a_plan() {
        for (status, history_relation) in [
            (
                "terminal_event_history_incomplete_preserve_and_hold",
                "terminal_event_history_incomplete",
            ),
            (
                "terminal_event_history_conflicting_preserve_and_hold",
                "terminal_event_history_conflicting",
            ),
        ] {
            let mut record = migration_admission_record(&admission());
            record.status = status.to_owned();
            record.history_relation = history_relation.to_owned();
            record.quarantine_plan = None;

            assert!(
                validate_accepted_record(
                    &record,
                    &format!("routine-quarantine-sha256:{}", "a".repeat(64))
                )
                .is_none()
            );
        }
    }

    #[test]
    fn typed_event_history_holds_reject_a_forged_apply_capable_plan() {
        for (status, history_relation) in [
            (
                "terminal_event_history_incomplete_preserve_and_hold",
                "terminal_event_history_incomplete",
            ),
            (
                "terminal_event_history_conflicting_preserve_and_hold",
                "terminal_event_history_conflicting",
            ),
        ] {
            let mut forged = migration_admission_record(&admission());
            forged.status = status.to_owned();
            forged.history_relation = history_relation.to_owned();
            let accepted = forged.quarantine_plan.as_ref().unwrap().plan_id.clone();

            assert!(validate_accepted_record(&forged, &accepted).is_none());
        }
    }

    #[test]
    fn abandonment_record_requires_v6_identity_bound_consequences_and_separate_validation() {
        let mut admission = admission();
        admission.status = "legacy_history_abandonment_requires_explicit_retirement_approval";
        admission.history_relation = "terminal_event_history_incomplete";
        let plan = admission.quarantine_plan.as_mut().unwrap();
        plan.schema_version = ABANDONMENT_PLAN_SCHEMA.to_owned();
        plan.history_relation = "terminal_event_history_incomplete".to_owned();
        plan.authoritative_history = "none_explicit_operator_abandonment".to_owned();
        let record = abandonment_record(&admission).unwrap();
        let accepted = record.quarantine_plan.plan_id.clone();

        assert_eq!(
            validate_abandonment_record(&record, &accepted),
            Some(&record.quarantine_plan)
        );
        assert!(
            validate_accepted_record(&migration_admission_record(&admission), &accepted).is_none()
        );

        let mut substituted = record.clone();
        substituted.consequences.previous_reuse_and_recovery = "available".to_owned();
        assert!(validate_abandonment_record(&substituted, &accepted).is_none());
    }

    #[test]
    fn previous_v6_admission_with_current_v5_plan_is_rejected_fail_closed() {
        let mut previous_admission = migration_admission_record(&admission());
        previous_admission.schema_version = "RoutineStateMigrationAdmission-v6".to_owned();
        let accepted = previous_admission
            .quarantine_plan
            .as_ref()
            .unwrap()
            .plan_id
            .clone();

        let previous_bytes = serde_json::to_vec(&previous_admission).unwrap();
        let decoded: RoutineStateMigrationRecord = serde_json::from_slice(&previous_bytes).unwrap();

        assert_eq!(
            decoded.quarantine_plan.as_ref().unwrap().schema_version,
            PLAN_SCHEMA
        );
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), previous_bytes);
        assert!(validate_accepted_record(&decoded, &accepted).is_none());
    }

    #[test]
    fn current_v7_admission_with_previous_v4_plan_is_rejected_fail_closed() {
        let mut previous_plan = migration_admission_record(&admission());
        previous_plan
            .quarantine_plan
            .as_mut()
            .unwrap()
            .schema_version = "RoutineStateQuarantinePlan-v4".to_owned();
        let accepted = previous_plan
            .quarantine_plan
            .as_ref()
            .unwrap()
            .plan_id
            .clone();

        let previous_bytes = serde_json::to_vec(&previous_plan).unwrap();
        let decoded: RoutineStateMigrationRecord = serde_json::from_slice(&previous_bytes).unwrap();

        assert_eq!(decoded.schema_version, ADMISSION_SCHEMA);
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), previous_bytes);
        assert!(validate_accepted_record(&decoded, &accepted).is_none());
    }

    #[test]
    fn exact_stale_reserved_no_effect_owner_absent_record_is_apply_capable() {
        let mut admission = admission();
        admission.status = "stale_reserved_abandoned_candidate_preserve_and_hold";
        admission.history_relation = "redundant_equivalent";
        let plan = admission.quarantine_plan.as_mut().unwrap();
        plan.history_relation = "redundant_equivalent".to_owned();
        plan.authoritative_history =
            "none_noncurrent_value_only_historical_head_unproven".to_owned();
        admission.reserved_recovery = Some(super::super::host::RoutineReservedRecoveryAdmission {
            status: "stale_reserved_abandoned_candidate_preserve_and_hold",
            owner_observation: "owner_process_not_observed",
            effect_evidence: "pristine_no_effect_observed",
        });
        let record = migration_admission_record(&admission);
        let accepted = record.quarantine_plan.as_ref().unwrap().plan_id.clone();
        assert!(validate_accepted_record(&record, &accepted).is_some());

        for mutation in ["owner", "effect", "authorization"] {
            let mut rejected = record.clone();
            let reserved = rejected.reserved_recovery.as_mut().unwrap();
            match mutation {
                "owner" => reserved.owner_observation = "exact_owner_process_observed".to_owned(),
                "effect" => reserved.effect_evidence = "effect_or_ambiguity_present".to_owned(),
                "authorization" => reserved.recovery_authorized = true,
                _ => unreachable!(),
            }
            assert!(validate_accepted_record(&rejected, &accepted).is_none());
        }
    }

    #[test]
    fn exact_record_admission_rejects_unknown_fields_and_one_axis_substitution() {
        let mut value = serde_json::to_value(migration_admission_record(&admission())).unwrap();
        value["unbound"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<RoutineStateMigrationRecord>(value).is_err());

        let record = migration_admission_record(&admission());
        assert!(validate_accepted_record(&record, "routine-quarantine-sha256:wrong").is_none());
        let mut wrong = record.clone();
        wrong.migration_authorized = true;
        let accepted = wrong.quarantine_plan.as_ref().unwrap().plan_id.clone();
        assert!(validate_accepted_record(&wrong, &accepted).is_none());
    }

    #[test]
    fn input_frame_accepts_only_canonical_bytes_with_at_most_one_terminal_lf() {
        assert_eq!(exact_json_frame(b"{}"), Some(&b"{}"[..]));
        assert_eq!(exact_json_frame(b"{}\n"), Some(&b"{}"[..]));
        assert_eq!(exact_json_frame(b"{}\n\n"), None);
    }

    #[test]
    fn post_apply_stale_context_preserves_the_observed_host_effect() {
        let outcome = project_apply_outcome(
            super::super::host::RoutineStateQuarantineApplyOutcome {
                status: "applied",
                effect: "staged_fresh_v8_then_atomically_exchanged_quarantined_legacy_and_published_settlement_receipt",
                settlement_state: "settled_receipt_durable",
                plan_id: format!("routine-quarantine-sha256:{}", "a".repeat(64)),
                quarantine_owner: format!("routine-public.quarantine-{}", "a".repeat(64)),
                fresh_format_verified: true,
            },
            false,
            false,
        );
        assert_eq!(outcome.exit_class, ExitClass::ActionableFinding);
        assert!(outcome.machine_payload.is_none());
        let diagnostic = serde_json::to_value(outcome.diagnostic.unwrap()).unwrap();
        assert_eq!(
            diagnostic["diagnostic_id"],
            "successor_runtime_stale_context"
        );
        assert_eq!(
            diagnostic["effect"],
            "staged_fresh_v8_then_atomically_exchanged_quarantined_legacy_and_published_settlement_receipt"
        );
    }
}
