//! Exact executable-control ledger for behavioral tests retired by `67eabfb0`.
//!
//! This is routing metadata, not proof. Each executable name is run separately;
//! broker-blocked rows name the exact post-authorization gap and a pre-broker
//! control that still runs on the fail-closed path.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Status {
    Executable(&'static [&'static str]),
    BrokerBlocked {
        cause: &'static str,
        pre_broker_controls: &'static [&'static str],
    },
}

pub(crate) struct Mapping {
    pub(crate) retired: &'static str,
    pub(crate) status: Status,
}

pub(crate) const CHILD_LIFECYCLE_BLOCKER: &str = "requires an opaque root-broker authorization before spawn; the fail-closed child route cannot reach post-spawn cancellation, timeout, setup, descendant, mapping, sandbox, or natural-exit observation";
pub(crate) const CHILD_SUCCESS_BLOCKER: &str = "requires an opaque root-broker authorization and immutable installed ultragoal runtime before a legitimate behavior result or reuse artifact can exist";

pub(crate) const fn executable(
    retired: &'static str,
    controls: &'static [&'static str],
) -> Mapping {
    Mapping {
        retired,
        status: Status::Executable(controls),
    }
}

pub(crate) const fn blocked(
    retired: &'static str,
    cause: &'static str,
    controls: &'static [&'static str],
) -> Mapping {
    Mapping {
        retired,
        status: Status::BrokerBlocked {
            cause,
            pre_broker_controls: controls,
        },
    }
}

#[test]
fn every_retired_behavior_has_a_unique_executable_or_causal_broker_row() {
    let rows = super::invariant_control_map_adapter::MAP
        .iter()
        .chain(super::invariant_control_map_authority::MAP)
        .chain(super::invariant_control_map_mediator_a::MAP)
        .chain(super::invariant_control_map_mediator_b::MAP)
        .collect::<Vec<_>>();
    let mut retired = rows.iter().map(|row| row.retired).collect::<Vec<_>>();
    retired.sort_unstable();
    retired.dedup();
    assert_eq!(retired.len(), rows.len(), "retired invariant mapped twice");
    for row in rows {
        match row.status {
            Status::Executable(controls) => assert!(!controls.is_empty(), "{}", row.retired),
            Status::BrokerBlocked {
                cause,
                pre_broker_controls,
            } => {
                assert!(
                    cause == CHILD_LIFECYCLE_BLOCKER || cause == CHILD_SUCCESS_BLOCKER,
                    "unbounded blocker: {}",
                    row.retired
                );
                assert!(!pre_broker_controls.is_empty(), "{}", row.retired);
            }
        }
    }
}
