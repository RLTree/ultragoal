use super::host::RoutineStateQuarantinePlan;
use super::*;
use crate::cli::successor::ExitClass;
use crate::cli::successor::command_contract::MigrateAction;
use crate::cli::successor::runtime::{Diagnostic, DiagnosticDetails, DiagnosticId};
use serde::{Deserialize, Serialize};

pub(crate) const ABANDONMENT_RECORD_SCHEMA: &str = "RoutineStateAbandonmentPlan-v1";
pub(crate) const ABANDONMENT_PLAN_SCHEMA: &str = "RoutineStateQuarantinePlan-v6";
pub(crate) const ABANDONMENT_SUPPORT_LIMIT: &str = "read-only exact-owner legacy history abandonment planning plus separately approved no-gap whole-owner quarantine; legacy continuity is abandoned, no legacy authority is imported, previous reuse and recovery are unavailable, and declared target-local routine outputs may re-execute; no HostState write, routine execution, ProductState, Product Fitness, readiness, installation, or release claim";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RoutineStateAbandonmentRecord {
    pub(crate) schema_version: String,
    pub(crate) status: String,
    pub(crate) format_status: String,
    pub(crate) legacy_singleton_count: usize,
    pub(crate) canonical_continuation_count: usize,
    pub(crate) event_journal_count: usize,
    pub(crate) history_relation: String,
    pub(crate) abandonment_effect: String,
    pub(crate) abandonment_authorized: bool,
    pub(crate) consequences: RoutineStateAbandonmentConsequences,
    pub(crate) quarantine_plan: RoutineStateQuarantinePlan,
    pub(crate) next_action: String,
    pub(crate) claim_effect: String,
    pub(crate) support_limit: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RoutineStateAbandonmentConsequences {
    pub(crate) legacy_history_continuity: String,
    pub(crate) legacy_authority_imported: bool,
    pub(crate) previous_reuse_and_recovery: String,
    pub(crate) possible_duplicate_effect: String,
}

pub(crate) fn abandonment_record(
    admission: &RoutineStateMigrationAdmission,
) -> Option<RoutineStateAbandonmentRecord> {
    let plan = admission.quarantine_plan.as_ref()?;
    let supported_relation = matches!(
        admission.history_relation,
        "terminal_event_history_incomplete"
            | "terminal_event_history_conflicting"
            | "conflicting_histories_hold"
    );
    if !supported_relation
        || admission.format_status != "absent_legacy"
        || admission.reserved_recovery.is_some()
        || plan.schema_version != ABANDONMENT_PLAN_SCHEMA
        || plan.history_relation != admission.history_relation
        || plan.authoritative_history != "none_explicit_operator_abandonment"
        || plan.apply_capability != "available_exact_record_only"
    {
        return None;
    }
    Some(RoutineStateAbandonmentRecord {
        schema_version: ABANDONMENT_RECORD_SCHEMA.to_owned(),
        status: "legacy_history_abandonment_requires_explicit_retirement_approval".to_owned(),
        format_status: admission.format_status.to_owned(),
        legacy_singleton_count: admission.legacy_singleton_count,
        canonical_continuation_count: admission.canonical_continuation_count,
        event_journal_count: admission.event_journal_count,
        history_relation: admission.history_relation.to_owned(),
        abandonment_effect: "none".to_owned(),
        abandonment_authorized: false,
        consequences: RoutineStateAbandonmentConsequences {
            legacy_history_continuity: "abandoned".to_owned(),
            legacy_authority_imported: false,
            previous_reuse_and_recovery: "unavailable".to_owned(),
            possible_duplicate_effect:
                "declared_target_local_routine_outputs_may_reexecute".to_owned(),
        },
        quarantine_plan: plan.clone(),
        next_action: "persist this exact JSON unchanged at a relative in-repository path; independently review the bound consequences, then run ultragoal --json migrate apply --plan <relative-abandonment-record> --accept-plan <quarantine_plan.plan_id> --approve-retirement".to_owned(),
        claim_effect: "none".to_owned(),
        support_limit: ABANDONMENT_SUPPORT_LIMIT.to_owned(),
    })
}

pub(crate) fn plan_state_abandonment(
    root: &Path,
    read_context: &LiveContext,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> RuntimeOutcome {
    let Some(requested_target) = target_argument(invocation) else {
        return failure(
            ExitClass::InvalidInvocation,
            DiagnosticId::UnexpectedArguments,
            "migrate abandon-plan accepts at most one canonical repository --target",
            "reparse the exact abandon-plan command through the successor grammar",
        );
    };
    let Some(home) = home else {
        return failure(
            ExitClass::BlockedAuthority,
            DiagnosticId::AuthorityRequired,
            "personal HostState authority is unavailable for read-only abandonment planning",
            "run with one explicit personal home authority",
        );
    };
    let binding = match current_diagnosis_binding(root, requested_target, read_context) {
        Ok(Some(binding)) => binding,
        _ => {
            return failure(
                ExitClass::ActionableFinding,
                DiagnosticId::ContextUnavailable,
                "the exact routine diagnosis binding is unavailable",
                "restore the exact routine manifest and source binding before planning abandonment",
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
                "stabilize the candidate and retry abandonment planning",
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
    let admission = match HostState::assess_abandonment_admission(home, checkpoint_binding) {
        Ok(Some(admission)) => admission,
        _ => {
            return failure(
                ExitClass::BlockedAuthority,
                DiagnosticId::AuthorityRequired,
                "the exact legacy owner is unavailable or unsafe for abandonment planning",
                "preserve HostState and resolve the typed hold without applying a quarantine",
            );
        }
    };
    let Some(record) = abandonment_record(&admission) else {
        return failure(
            ExitClass::ActionableFinding,
            DiagnosticId::AuthorityRequired,
            "current evidence does not support destructive legacy-history abandonment",
            "use ordinary diagnose when lawful migration or recovery remains available; otherwise preserve the typed hold",
        );
    };
    if read_context.revalidate().is_err() || binding.context().revalidate().is_err() {
        return failure(
            ExitClass::ActionableFinding,
            DiagnosticId::StaleContext,
            "the repository candidate changed while abandonment was being planned",
            "discard the stale plan and retry from the stable exact candidate",
        );
    }
    match serde_json::to_vec(&record) {
        Ok(machine) if super::super::public_output_allowed(machine.len()) => {
            RuntimeOutcome::payload(
                ExitClass::ActionableFinding,
                machine,
                "legacy Routine history abandonment requires explicit retirement approval"
                    .to_owned(),
            )
        }
        _ => failure(
            ExitClass::InternalFailure,
            DiagnosticId::ProjectionFailed,
            "the bounded abandonment plan could not be encoded",
            "preserve HostState and retry from the exact source candidate",
        ),
    }
}

fn target_argument(invocation: &ParsedInvocation) -> Option<Option<&str>> {
    if invocation.command != SuccessorCommand::Migrate(MigrateAction::Abandon)
        || invocation.effect != EffectClass::Read
        || invocation.arguments.len() > 1
    {
        return None;
    }
    match invocation.arguments.as_slice() {
        [] => Some(None),
        [argument] if argument.name == OptionName::Target => match &argument.value {
            ParsedValue::RepositoryTarget(target) => Some(Some(target.as_str())),
            _ => None,
        },
        _ => None,
    }
}

fn failure(
    class: ExitClass,
    id: DiagnosticId,
    cause: &'static str,
    repair: &'static str,
) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        class,
        Diagnostic::new(
            id,
            class,
            DiagnosticDetails {
                cause,
                affected_surface: "routine HostState abandonment plan",
                repair,
                effect: "none",
                rerun: "ultragoal --json migrate abandon-plan [--target <repository>]",
                ceiling: ABANDONMENT_SUPPORT_LIMIT,
            },
        ),
    )
}
