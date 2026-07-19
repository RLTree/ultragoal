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
