use serde::Serialize;

pub(crate) const PERFORMANCE_RECEIPT_SCHEMA: &str = "harness-ultragoal.cli-performance-receipt.v1";

pub(crate) const PERFORMANCE_BUDGET_VERSION: &str = "2026-06-26.gate-89-22.v1";

pub(crate) const PERFORMANCE_COMMANDS: &[&str] = &[
    "ultragoal performance prove",
    "ultragoal performance verify",
    "ultragoal performance budgets",
    "ultragoal self performance prove",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PerformanceOperation {
    Prove,
    Verify,
    Budgets,
    SelfProve,
}

impl PerformanceOperation {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Prove => "performance_prove",
            Self::Verify => "performance_verify",
            Self::Budgets => "performance_budgets",
            Self::SelfProve => "self_performance_prove",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BudgetClass {
    HotEditCheck,
    FocusedRepair,
    StandardSourceLocal,
    StrictLocal,
    StrictFixtures,
    StrictCoverage,
    StrictFinal,
    ExternalLive,
}

impl BudgetClass {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::HotEditCheck => "hot_edit_check",
            Self::FocusedRepair => "focused_repair",
            Self::StandardSourceLocal => "standard_source_local",
            Self::StrictLocal => "strict_local",
            Self::StrictFixtures => "strict_fixtures",
            Self::StrictCoverage => "strict_coverage",
            Self::StrictFinal => "strict_final",
            Self::ExternalLive => "external_live",
        }
    }

    pub(crate) fn from_str(raw: &str) -> Option<Self> {
        match raw {
            "hot" | "hot_edit_check" | "hot-edit-check" | "instant" | "interactive" => {
                Some(Self::HotEditCheck)
            }
            "focused" | "focused_repair" | "focused-repair" => Some(Self::FocusedRepair),
            "standard"
            | "source-local"
            | "source_local"
            | "standard_source_local"
            | "standard-source-local"
            | "repair_loop"
            | "repair-loop" => Some(Self::StandardSourceLocal),
            "strict_local" | "strict-local" => Some(Self::StrictLocal),
            "strict_fixtures" | "strict-fixtures" => Some(Self::StrictFixtures),
            "strict_coverage" | "strict-coverage" => Some(Self::StrictCoverage),
            "strict_final" | "strict-final" => Some(Self::StrictFinal),
            "external_live" | "external-live" => Some(Self::ExternalLive),
            _ => None,
        }
    }

    pub(crate) fn cold_p95_ms(self) -> u64 {
        match self {
            Self::HotEditCheck => 5_000,
            Self::FocusedRepair => 15_000,
            Self::StandardSourceLocal => 30_000,
            Self::StrictLocal => 60_000,
            Self::StrictFixtures => 60_000,
            Self::StrictCoverage => 60_000,
            Self::StrictFinal => 60_000,
            Self::ExternalLive => 30_000,
        }
    }

    pub(crate) fn warm_p95_ms(self) -> Option<u64> {
        match self {
            Self::HotEditCheck => Some(5_000),
            Self::FocusedRepair => Some(5_000),
            Self::StandardSourceLocal => Some(10_000),
            _ => None,
        }
    }

    pub(crate) fn hard_ceiling_ms(self) -> u64 {
        match self {
            Self::HotEditCheck => 5_000,
            Self::FocusedRepair => 15_000,
            Self::StandardSourceLocal => 60_000,
            Self::StrictLocal | Self::StrictFixtures | Self::StrictCoverage | Self::StrictFinal => {
                180_000
            }
            Self::ExternalLive => 30_000,
        }
    }
}
