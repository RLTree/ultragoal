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
    Instant,
    Interactive,
    Focused,
    RepairLoop,
    StrictLocal,
    StrictFixtures,
    StrictCoverage,
    StrictFinal,
    ExternalLive,
}

impl BudgetClass {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Instant => "instant",
            Self::Interactive => "interactive",
            Self::Focused => "focused",
            Self::RepairLoop => "repair_loop",
            Self::StrictLocal => "strict_local",
            Self::StrictFixtures => "strict_fixtures",
            Self::StrictCoverage => "strict_coverage",
            Self::StrictFinal => "strict_final",
            Self::ExternalLive => "external_live",
        }
    }

    pub(crate) fn from_str(raw: &str) -> Option<Self> {
        match raw {
            "instant" => Some(Self::Instant),
            "interactive" => Some(Self::Interactive),
            "focused" => Some(Self::Focused),
            "repair_loop" | "repair-loop" => Some(Self::RepairLoop),
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
            Self::Instant => 2_000,
            Self::Interactive => 5_000,
            Self::Focused => 15_000,
            Self::RepairLoop => 30_000,
            Self::StrictLocal => 60_000,
            Self::StrictFixtures => 120_000,
            Self::StrictCoverage => 300_000,
            Self::StrictFinal => 600_000,
            Self::ExternalLive => 30_000,
        }
    }

    pub(crate) fn warm_p95_ms(self) -> Option<u64> {
        match self {
            Self::Instant => Some(500),
            Self::Interactive => Some(1_000),
            Self::Focused => Some(5_000),
            Self::RepairLoop => Some(10_000),
            _ => None,
        }
    }
}
