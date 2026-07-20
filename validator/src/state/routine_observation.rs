use super::finding::Finding;
use serde::{Deserialize, Serialize};

/// Parent-owned causal identity captured before a routine effect. A binding is
/// absent when the pre-effect state is empty or ambiguous, so event position
/// can never manufacture causality.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutineFindingBinding {
    pub finding_id: String,
    pub repair_id: String,
}

impl RoutineFindingBinding {
    pub(crate) fn from_findings(findings: &[Finding]) -> Option<Self> {
        let [finding] = findings else {
            return None;
        };
        Some(Self {
            finding_id: finding.finding_id.clone(),
            repair_id: finding.repair.repair_id.clone(),
        })
    }

    pub(crate) fn matches(&self, finding: &Finding) -> bool {
        self.finding_id == finding.finding_id && self.repair_id == finding.repair.repair_id
    }
}

/// Diagnostic evidence associated with a terminal routine event.
///
/// This is intentionally outside `findings`, `repairs`, and claim ceilings:
/// the event may help an operator explain interruption, recovery, or reuse,
/// but it cannot create or settle a product finding.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RoutineFindingObservation {
    pub event_id: String,
    pub continuation_id: String,
    pub terminal_ledger_head: String,
    pub finding_id: String,
    pub repair_id: String,
    pub transition: RoutineObservationTransition,
    pub outcome: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoutineObservationTransition {
    Interrupted,
    Recovered,
    Reused,
    Executed,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoutineObservationWindow {
    NotQueried,
    Absent,
    Available,
    Saturated,
    Unavailable,
}
