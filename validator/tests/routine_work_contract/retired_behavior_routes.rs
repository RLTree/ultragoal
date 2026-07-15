//! Exact routed-control ledger for behavioral tests retired by `67eabfb0`.
//!
//! Routed names are unverified candidates, not proof. The sole
//! `ExecutedEquivalent` row invokes its bound control in this target;
//! local issuer-blocked rows name an unavailable post-authorization surface.

#[derive(Clone, Copy)]
pub(crate) enum Status {
    ExecutedEquivalent {
        control_id: &'static str,
        execute: fn(),
    },
    Routed(&'static [&'static str]),
    ExternalBlocked {
        cause: &'static str,
        pre_authority_controls: &'static [&'static str],
    },
}

pub(crate) struct Mapping {
    pub(crate) retired: &'static str,
    pub(crate) status: Status,
}

pub(crate) const CHILD_LIFECYCLE_BLOCKER: &str = "requires an opaque local-issuer authorization before spawn; the fail-closed child route cannot reach post-spawn cancellation, timeout, setup, descendant, mapping, sandbox, or natural-exit observation";
pub(crate) const CHILD_SUCCESS_BLOCKER: &str = "requires an opaque local-issuer authorization and immutable installed ultragoal runtime before a legitimate behavior result or reuse artifact can exist";

pub(crate) const fn routed(retired: &'static str, controls: &'static [&'static str]) -> Mapping {
    Mapping {
        retired,
        status: Status::Routed(controls),
    }
}

pub(crate) const fn executed(
    retired: &'static str,
    control_id: &'static str,
    execute: fn(),
) -> Mapping {
    Mapping {
        retired,
        status: Status::ExecutedEquivalent {
            control_id,
            execute,
        },
    }
}

pub(crate) const fn blocked(
    retired: &'static str,
    cause: &'static str,
    controls: &'static [&'static str],
) -> Mapping {
    Mapping {
        retired,
        status: Status::ExternalBlocked {
            cause,
            pre_authority_controls: controls,
        },
    }
}

#[test]
fn retired_behavior_routes_preserve_exact_claim_ceiling() {
    let rows = super::retired_adapter_routes::MAP
        .iter()
        .chain(super::retired_authority_routes::MAP)
        .chain(super::retired_process_lifecycle_routes::MAP)
        .chain(super::retired_reuse_output_routes::MAP)
        .collect::<Vec<_>>();
    let mut retired = rows.iter().map(|row| row.retired).collect::<Vec<_>>();
    retired.sort_unstable();
    retired.dedup();
    assert_eq!(retired.len(), rows.len(), "retired invariant mapped twice");
    let mut executed_count = 0;
    let mut routed_count = 0;
    let mut blocked_count = 0;
    for row in rows {
        match row.status {
            Status::ExecutedEquivalent {
                control_id,
                execute,
            } => {
                executed_count += 1;
                assert_eq!(
                    control_id,
                    "issuer_api_visibility::sealed_issuer_and_grant_entrypoints_are_not_externally_callable"
                );
                execute();
            }
            Status::Routed(controls) => {
                routed_count += 1;
                assert!(!controls.is_empty(), "{}", row.retired);
            }
            Status::ExternalBlocked {
                cause,
                pre_authority_controls,
            } => {
                blocked_count += 1;
                assert!(
                    cause == CHILD_LIFECYCLE_BLOCKER || cause == CHILD_SUCCESS_BLOCKER,
                    "unbounded blocker: {}",
                    row.retired
                );
                assert!(!pre_authority_controls.is_empty(), "{}", row.retired);
            }
        }
    }
    assert_eq!((executed_count, routed_count, blocked_count), (1, 41, 23));
}
