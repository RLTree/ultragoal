use super::super::super::ExitClass;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DiagnosticId {
    EffectMismatch,
    UnexpectedArguments,
    ContextUnavailable,
    InventoryUnavailable,
    ObservabilityUnavailable,
    StaleContext,
    StateUnavailable,
    StateContextMismatch,
    FindingNotPresent,
    ProjectionFailed,
    RepositoryFitRequired,
    DownstreamToolUnavailable,
    AuthorityRequired,
}

impl DiagnosticId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EffectMismatch => "successor_runtime_effect_mismatch",
            Self::UnexpectedArguments => "successor_runtime_unexpected_arguments",
            Self::ContextUnavailable => "successor_runtime_context_unavailable",
            Self::InventoryUnavailable => "successor_runtime_inventory_unavailable",
            Self::ObservabilityUnavailable => "successor_runtime_observability_unavailable",
            Self::StaleContext => "successor_runtime_stale_context",
            Self::StateUnavailable => "successor_runtime_state_unavailable",
            Self::StateContextMismatch => "successor_runtime_state_context_mismatch",
            Self::FindingNotPresent => "successor_runtime_finding_not_present",
            Self::ProjectionFailed => "successor_runtime_projection_failed",
            Self::RepositoryFitRequired => "repository_fit_required",
            Self::DownstreamToolUnavailable => "successor_runtime_downstream_tool_unavailable",
            Self::AuthorityRequired => "successor_runtime_authority_required",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct Diagnostic {
    schema_version: &'static str,
    diagnostic_id: &'static str,
    exit_class: &'static str,
    cause: &'static str,
    affected_surface: &'static str,
    smallest_safe_repair: &'static str,
    effect: &'static str,
    exact_rerun: &'static str,
    resulting_ceiling: &'static str,
}

pub(crate) struct DiagnosticDetails {
    pub(crate) cause: &'static str,
    pub(crate) affected_surface: &'static str,
    pub(crate) repair: &'static str,
    pub(crate) effect: &'static str,
    pub(crate) rerun: &'static str,
    pub(crate) ceiling: &'static str,
}

impl Diagnostic {
    pub const fn new(id: DiagnosticId, exit_class: ExitClass, details: DiagnosticDetails) -> Self {
        Self {
            schema_version: "HarnessDiagnostic-v1",
            diagnostic_id: id.as_str(),
            exit_class: exit_name(exit_class),
            cause: details.cause,
            affected_surface: details.affected_surface,
            smallest_safe_repair: details.repair,
            effect: details.effect,
            exact_rerun: details.rerun,
            resulting_ceiling: details.ceiling,
        }
    }

    pub fn human(&self) -> String {
        format!(
            "{}: {}\nrepair: {}\nrerun: {}\nceiling: {}",
            self.diagnostic_id,
            self.cause,
            self.smallest_safe_repair,
            self.exact_rerun,
            self.resulting_ceiling
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeOutcome {
    pub exit_class: ExitClass,
    pub machine_payload: Option<Vec<u8>>,
    pub human_payload: Option<String>,
    pub diagnostic: Option<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeStreams {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl RuntimeOutcome {
    pub fn payload(exit_class: ExitClass, machine: Vec<u8>, human: String) -> Self {
        Self {
            exit_class,
            machine_payload: Some(machine),
            human_payload: Some(human),
            diagnostic: None,
        }
    }

    pub const fn failure(exit_class: ExitClass, diagnostic: Diagnostic) -> Self {
        Self {
            exit_class,
            machine_payload: None,
            human_payload: None,
            diagnostic: Some(diagnostic),
        }
    }
}

const fn exit_name(class: ExitClass) -> &'static str {
    match class {
        ExitClass::Success => "success",
        ExitClass::ActionableFinding => "actionable_finding",
        ExitClass::InvalidInvocation => "invalid_invocation",
        ExitClass::BlockedAuthority => "blocked_authority",
        ExitClass::UnsupportedCapability => "unsupported_capability",
        ExitClass::InternalFailure => "internal_failure",
    }
}
