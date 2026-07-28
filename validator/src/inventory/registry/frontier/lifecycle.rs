use crate::inventory::types::InventoryError;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct ExpectedLifecycle {
    pub(super) ready: BTreeSet<String>,
    pub(super) active_worktree_lanes: BTreeSet<String>,
}

pub(super) fn expected(
    frontier: &str,
    states: &BTreeMap<String, String>,
) -> Result<ExpectedLifecycle, InventoryError> {
    let (ready, active_worktree_lanes) = match frontier {
        "N00_ADOPTION_BOUNDARY" => (&["N01"][..], &[][..]),
        "N01_INTEGRATED" => (&["N02"][..], &[][..]),
        "N03_INTEGRATED_DEBT_CHECKPOINT" | "N08_N09_INTEGRATED_DEBT_CHECKPOINT" => {
            (&[][..], &[][..])
        }
        "N04_N07_READY_SOURCE_FRONTIER" => (&["N04", "N05", "N06", "N07"][..], &[][..]),
        "N05_N07_READY_N04_INTEGRATED_SOURCE_FRONTIER" => (&["N05", "N06", "N07"][..], &[][..]),
        "N06_N07_READY_N04_N05_INTEGRATED_SOURCE_FRONTIER" => (&["N06", "N07"][..], &[][..]),
        "N07_READY_N04_N06_INTEGRATED_SOURCE_FRONTIER" => (&["N07"][..], &[][..]),
        "N08_N09_READY_N07_INTEGRATED_SOURCE_FRONTIER" => (&["N08", "N09"][..], &[][..]),
        "N08_READY_N09_INTEGRATED_SOURCE_FRONTIER" => (&["N08"][..], &[][..]),
        "N10_ROOT_PLANNED_N11_READY_SOURCE_FRONTIER" => {
            exact_state(states, "N10", "planned")?;
            exact_state(states, "N11", "ready")?;
            (&["N11"][..], &[][..])
        }
        "N10_ROOT_PLANNED_N11_ACTIVE_SOURCE_FRONTIER" => {
            exact_state(states, "N10", "planned")?;
            active_state(states, "N11")?;
            (&[][..], &["N11"][..])
        }
        "N10_INTEGRATED_N11_ACTIVE_SOURCE_FRONTIER" => {
            exact_state(states, "N10", "integrated")?;
            active_state(states, "N11")?;
            (&[][..], &["N11"][..])
        }
        "N10_ROOT_PLANNED_N11_INTEGRATING_ROOT_CLOSURE" => {
            exact_state(states, "N10", "planned")?;
            exact_state(states, "N11", "integrating")?;
            (&[][..], &[][..])
        }
        "N10_INTEGRATED_N11_INTEGRATING_ROOT_CLOSURE" => {
            exact_state(states, "N10", "integrated")?;
            exact_state(states, "N11", "integrating")?;
            (&[][..], &[][..])
        }
        "N11_EXTERNAL_BLOCKED_N12_INTEGRATING_SOURCE_ACCEPTED" => {
            exact_state(states, "N10", "integrated")?;
            exact_state(states, "N11", "blocked")?;
            exact_state(states, "N12", "integrating")?;
            exact_state(states, "N14", "blocked")?;
            (&[][..], &[][..])
        }
        "N02_REOBSERVED_N12_INTEGRATED_N14_READY_SOURCE_FRONTIER" => {
            exact_state(states, "N10", "integrated")?;
            exact_state(states, "N11", "blocked")?;
            exact_state(states, "N12", "integrated")?;
            exact_state(states, "N14", "ready")?;
            (&["N14"][..], &[][..])
        }
        "N14_ACTIVE_N12_INTEGRATED_SOURCE_FRONTIER" => {
            exact_state(states, "N10", "integrated")?;
            exact_state(states, "N11", "blocked")?;
            exact_state(states, "N12", "integrated")?;
            active_state(states, "N14")?;
            (&[][..], &["N14"][..])
        }
        "N14_EXTERNAL_BLOCKED_N12_INTEGRATED_SOURCE_ACCEPTED" => {
            exact_state(states, "N10", "integrated")?;
            exact_state(states, "N11", "blocked")?;
            exact_state(states, "N12", "integrated")?;
            exact_state(states, "N14", "blocked")?;
            exact_state(states, "N15", "blocked")?;
            (&[][..], &[][..])
        }
        "N04_REPAIR_READY_N14_EXTERNAL_BLOCKED_N12_INTEGRATED_SOURCE_ACCEPTED" => {
            exact_state(states, "N04", "ready")?;
            repair_dependents(states)?;
            (&["N04"][..], &[][..])
        }
        "N04_REPAIR_ACTIVE_N14_EXTERNAL_BLOCKED_N12_INTEGRATED_SOURCE_ACCEPTED" => {
            active_state(states, "N04")?;
            repair_dependents(states)?;
            (&[][..], &["N04"][..])
        }
        "N04_REPAIR_INTEGRATED_DEPENDENTS_REOBSERVATION_REQUIRED" => {
            exact_state(states, "N04", "integrated")?;
            repair_dependents(states)?;
            (&[][..], &[][..])
        }
        "N08_REPAIR_READY_N14_EXTERNAL_BLOCKED_N12_REOBSERVATION_REQUIRED" => {
            exact_state(states, "N04", "integrated")?;
            exact_state(states, "N08", "ready")?;
            exact_state(states, "N11", "blocked")?;
            exact_state(states, "N12", "blocked")?;
            exact_state(states, "N14", "blocked")?;
            (&["N08"][..], &[][..])
        }
        "N08_REPAIR_ACTIVE_N14_EXTERNAL_BLOCKED_N12_REOBSERVATION_REQUIRED" => {
            exact_state(states, "N04", "integrated")?;
            active_state(states, "N08")?;
            exact_state(states, "N11", "blocked")?;
            exact_state(states, "N12", "blocked")?;
            exact_state(states, "N14", "blocked")?;
            (&[][..], &["N08"][..])
        }
        _ => return Err(invalid("scheduler frontier is unknown")),
    };
    Ok(ExpectedLifecycle {
        ready: named(ready),
        active_worktree_lanes: named(active_worktree_lanes),
    })
}

fn repair_dependents(states: &BTreeMap<String, String>) -> Result<(), InventoryError> {
    for lane in ["N08", "N11", "N12", "N14", "N15", "N16", "N17"] {
        exact_state(states, lane, "blocked")?;
    }
    Ok(())
}

fn exact_state(
    states: &BTreeMap<String, String>,
    lane: &str,
    expected: &str,
) -> Result<(), InventoryError> {
    if states.get(lane).map(String::as_str) != Some(expected) {
        return Err(invalid("scheduler frontier has an illegal lane lifecycle"));
    }
    Ok(())
}

fn active_state(states: &BTreeMap<String, String>, lane: &str) -> Result<(), InventoryError> {
    if !matches!(
        states.get(lane).map(String::as_str),
        Some("leased" | "candidate" | "under_review" | "rework" | "accepted")
    ) {
        return Err(invalid(
            "scheduler frontier has an illegal active lane lifecycle",
        ));
    }
    Ok(())
}

fn named(lanes: &[&str]) -> BTreeSet<String> {
    lanes.iter().map(|lane| (*lane).to_owned()).collect()
}

fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}
