use super::*;

#[test]
fn migration_blocker_keeps_retirement_and_scheduler_blocked() {
    let nodes = scheduler_nodes(&registry(
        &[
            ("N10", "integrated"),
            ("N11", "blocked"),
            ("N12", "integrated"),
            ("N14", "blocked"),
            ("N15", "blocked"),
        ],
        &[],
        "N14_EXTERNAL_BLOCKED_N12_INTEGRATED_SOURCE_ACCEPTED",
    ))
    .unwrap();

    assert!(nodes.ready.is_empty());
    assert!(nodes.active_worktree_lanes.is_empty());
}

#[test]
fn migration_blocker_cannot_activate_retirement() {
    for state in ["ready", "leased"] {
        let eligible = if state == "ready" {
            &["N15"][..]
        } else {
            &[][..]
        };
        let result = scheduler_nodes(&registry(
            &[
                ("N10", "integrated"),
                ("N11", "blocked"),
                ("N12", "integrated"),
                ("N14", "blocked"),
                ("N15", state),
            ],
            eligible,
            "N14_EXTERNAL_BLOCKED_N12_INTEGRATED_SOURCE_ACCEPTED",
        ));
        assert!(result.is_err(), "N15 {state}");
    }
}

#[test]
fn late_distribution_repair_has_one_ready_then_one_active_lane() {
    let ready = scheduler_nodes(&registry(
        &repair_states("ready"),
        &["N04"],
        "N04_REPAIR_READY_N14_EXTERNAL_BLOCKED_N12_INTEGRATED_SOURCE_ACCEPTED",
    ))
    .unwrap();
    assert_eq!(ready.ready, ["N04".to_owned()].into());
    assert!(ready.active_worktree_lanes.is_empty());

    let active = scheduler_nodes(&registry(
        &repair_states("leased"),
        &[],
        "N04_REPAIR_ACTIVE_N14_EXTERNAL_BLOCKED_N12_INTEGRATED_SOURCE_ACCEPTED",
    ))
    .unwrap();
    assert!(active.ready.is_empty());
    assert_eq!(active.active_worktree_lanes, ["N04".to_owned()].into());
}

#[test]
fn late_distribution_repair_keeps_downstream_lanes_blocked() {
    for lane in ["N08", "N11", "N12", "N14", "N15", "N16", "N17"] {
        let mut states = repair_states("ready");
        states.retain(|(id, _)| id != &lane);
        states.push((lane, "ready"));
        assert!(
            scheduler_nodes(&registry(
                &states,
                &["N04"],
                "N04_REPAIR_READY_N14_EXTERNAL_BLOCKED_N12_INTEGRATED_SOURCE_ACCEPTED",
            ))
            .is_err()
        );
    }
}

fn repair_states(n04: &'static str) -> Vec<(&'static str, &'static str)> {
    vec![
        ("N04", n04),
        ("N08", "blocked"),
        ("N11", "blocked"),
        ("N12", "blocked"),
        ("N14", "blocked"),
        ("N15", "blocked"),
        ("N16", "blocked"),
        ("N17", "blocked"),
    ]
}
