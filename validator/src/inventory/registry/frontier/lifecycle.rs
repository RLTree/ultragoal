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
        "N00_ADOPTION_BOUNDARY" => (&["N01"][..], &["N01"][..]),
        "N01_INTEGRATED" => (&["N02"][..], &["N02"][..]),
        "N03_INTEGRATED_DEBT_CHECKPOINT" | "N08_N09_INTEGRATED_DEBT_CHECKPOINT" => {
            (&[][..], &[][..])
        }
        "N04_N07_READY_SOURCE_FRONTIER" => (
            &["N04", "N05", "N06", "N07"][..],
            &["N04", "N05", "N06", "N07"][..],
        ),
        "N05_N07_READY_N04_INTEGRATED_SOURCE_FRONTIER" => {
            (&["N05", "N06", "N07"][..], &["N05", "N06", "N07"][..])
        }
        "N06_N07_READY_N04_N05_INTEGRATED_SOURCE_FRONTIER" => {
            (&["N06", "N07"][..], &["N06", "N07"][..])
        }
        "N07_READY_N04_N06_INTEGRATED_SOURCE_FRONTIER" => (&["N07"][..], &["N07"][..]),
        "N08_N09_READY_N07_INTEGRATED_SOURCE_FRONTIER" => {
            (&["N08", "N09"][..], &["N08", "N09"][..])
        }
        "N08_READY_N09_INTEGRATED_SOURCE_FRONTIER" => (&["N08"][..], &["N08"][..]),
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
        _ => return Err(invalid("scheduler frontier is unknown")),
    };
    Ok(ExpectedLifecycle {
        ready: named(ready),
        active_worktree_lanes: named(active_worktree_lanes),
    })
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
