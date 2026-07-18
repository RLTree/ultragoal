use super::scheduler_nodes;
use serde_json::json;
use std::collections::BTreeSet;

fn registry(states: &[(&str, &str)], eligible: &[&str]) -> serde_json::Value {
    json!({
        "pre_adoption_source": {
            "epoch": "ADOPTED-CURRENT",
            "frontier": "N00_ADOPTION_BOUNDARY",
            "eligible_scheduler_nodes": eligible,
        },
        "defaults": {"consumed": {"status": "resolved_current"}},
        "lanes": states
            .iter()
            .map(|(id, state)| json!({"id": id, "state": state}))
            .collect::<Vec<_>>(),
    })
}

#[test]
fn ready_lane_is_schedulable_without_becoming_production_authority() {
    let active = scheduler_nodes(&registry(
        &[("N00", "integrated"), ("N01", "ready")],
        &["N01"],
    ))
    .unwrap();

    assert_eq!(active.integrated, BTreeSet::from(["N00".to_owned()]));
}

#[test]
fn unintegrated_lifecycle_states_do_not_enter_the_production_frontier() {
    let active = scheduler_nodes(&registry(
        &[
            ("N00", "integrated"),
            ("N01", "ready"),
            ("N02", "leased"),
            ("N03", "candidate"),
            ("N04", "under_review"),
            ("N05", "accepted"),
            ("N06", "integrating"),
        ],
        &["N01"],
    ))
    .unwrap();

    assert_eq!(active.integrated, BTreeSet::from(["N00".to_owned()]));
}

#[test]
fn scheduler_eligibility_must_still_match_ready_lanes_exactly() {
    let error = scheduler_nodes(&registry(&[("N00", "integrated"), ("N01", "ready")], &[]))
        .err()
        .expect("mismatched eligibility must fail");

    assert_eq!(
        error.to_string(),
        "invalid registry: scheduler eligibility disagrees with ready lanes"
    );
}

#[test]
fn integrated_distribution_advances_only_the_remaining_ready_lanes() {
    let mut value = registry(
        &[
            ("N00", "integrated"),
            ("N01", "integrated"),
            ("N02", "integrated"),
            ("N03", "integrated"),
            ("N04", "integrated"),
            ("N05", "ready"),
            ("N06", "ready"),
            ("N07", "ready"),
        ],
        &["N05", "N06", "N07"],
    );
    value["pre_adoption_source"]["frontier"] =
        json!("N05_N07_READY_N04_INTEGRATED_SOURCE_FRONTIER");

    let nodes = scheduler_nodes(&value).unwrap();

    assert!(nodes.integrated.contains("N04"));
    assert_eq!(
        nodes.ready,
        BTreeSet::from(["N05", "N06", "N07"].map(str::to_owned))
    );
}

#[test]
fn integrated_repository_fit_preserves_the_two_actively_leased_core_lanes() {
    let mut value = registry(
        &[
            ("N00", "integrated"),
            ("N01", "integrated"),
            ("N02", "integrated"),
            ("N03", "integrated"),
            ("N04", "integrated"),
            ("N05", "integrated"),
            ("N06", "ready"),
            ("N07", "ready"),
        ],
        &["N06", "N07"],
    );
    value["pre_adoption_source"]["frontier"] =
        json!("N06_N07_READY_N04_N05_INTEGRATED_SOURCE_FRONTIER");

    let nodes = scheduler_nodes(&value).unwrap();

    assert!(nodes.integrated.contains("N05"));
    assert_eq!(
        nodes.ready,
        BTreeSet::from(["N06", "N07"].map(str::to_owned))
    );
}

#[test]
fn integrated_routine_leaves_only_observability_scheduler_ready() {
    let mut value = registry(
        &[
            ("N00", "integrated"),
            ("N01", "integrated"),
            ("N02", "integrated"),
            ("N03", "integrated"),
            ("N04", "integrated"),
            ("N05", "integrated"),
            ("N06", "integrated"),
            ("N07", "ready"),
        ],
        &["N07"],
    );
    value["pre_adoption_source"]["frontier"] =
        json!("N07_READY_N04_N06_INTEGRATED_SOURCE_FRONTIER");

    let nodes = scheduler_nodes(&value).unwrap();

    assert!(nodes.integrated.contains("N06"));
    assert_eq!(nodes.ready, BTreeSet::from(["N07".to_owned()]));
}

#[test]
fn integrated_observability_opens_only_plugin_and_agent_adoption() {
    let mut value = registry(
        &[
            ("N00", "integrated"),
            ("N01", "integrated"),
            ("N02", "integrated"),
            ("N03", "integrated"),
            ("N04", "integrated"),
            ("N05", "integrated"),
            ("N06", "integrated"),
            ("N07", "integrated"),
            ("N08", "ready"),
            ("N09", "ready"),
        ],
        &["N08", "N09"],
    );
    value["pre_adoption_source"]["frontier"] =
        json!("N08_N09_READY_N07_INTEGRATED_SOURCE_FRONTIER");

    let nodes = scheduler_nodes(&value).unwrap();

    assert!(nodes.integrated.contains("N07"));
    assert_eq!(
        nodes.ready,
        BTreeSet::from(["N08", "N09"].map(str::to_owned))
    );
}
