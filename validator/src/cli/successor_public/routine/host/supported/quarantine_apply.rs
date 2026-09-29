use super::super::{
    CheckpointBinding, RoutineStateMigrationAdmission, RoutineStateQuarantineApplyOutcome,
    RoutineStateQuarantinePlan,
};
use super::*;

pub(crate) fn apply_quarantine_plan(
    home_path: &Path,
    target: &Path,
    binding: CheckpointBinding<'_>,
    accepted: &RoutineStateQuarantinePlan,
    record_matches: &impl Fn(&RoutineStateMigrationAdmission) -> bool,
) -> Result<RoutineStateQuarantineApplyOutcome, HostFailure> {
    apply_plan(
        home_path,
        target,
        binding,
        accepted,
        record_matches,
        AdmissionMode::Migration,
    )
}

pub(crate) fn apply_abandonment_plan(
    home_path: &Path,
    target: &Path,
    binding: CheckpointBinding<'_>,
    accepted: &RoutineStateQuarantinePlan,
    record_matches: &impl Fn(&RoutineStateMigrationAdmission) -> bool,
) -> Result<RoutineStateQuarantineApplyOutcome, HostFailure> {
    apply_plan(
        home_path,
        target,
        binding,
        accepted,
        record_matches,
        AdmissionMode::ExplicitAbandonment,
    )
}

#[derive(Clone, Copy)]
enum AdmissionMode {
    Migration,
    ExplicitAbandonment,
}

fn apply_plan(
    home_path: &Path,
    target: &Path,
    binding: CheckpointBinding<'_>,
    accepted: &RoutineStateQuarantinePlan,
    record_matches: &impl Fn(&RoutineStateMigrationAdmission) -> bool,
    mode: AdmissionMode,
) -> Result<RoutineStateQuarantineApplyOutcome, HostFailure> {
    if binding.target() != target {
        return Err(HostFailure::Invalid);
    }
    host_state::validate_target(home_path, target)?;
    let home = host_state::open_home(home_path)?;
    let codex = home.open_child(STATE_COMPONENTS[0])?;
    let state_root = codex.open_child(STATE_COMPONENTS[1])?;
    let parent = state_root.open_child(STATE_COMPONENTS[2])?;
    let parent_lock = ParentStateLock::exclusive(&parent)?;
    let source = parent.open_child(STATE_COMPONENTS[3])?;
    let quarantine_present = parent.stat(&accepted.quarantine_owner)?.is_some();
    let bootstrap_present = parent.stat(BOOTSTRAP_STAGE)?.is_some();
    if source.stat(STATE_FORMAT_NAME)?.is_some() {
        return apply_from_fresh_source(
            record_matches,
            home,
            parent,
            parent_lock,
            source,
            binding,
            accepted,
            quarantine_present,
            bootstrap_present,
            mode,
        );
    }
    if quarantine_present {
        return Err(HostFailure::Invalid);
    }
    let ignored = bootstrap_present
        .then_some(BOOTSTRAP_STAGE)
        .into_iter()
        .collect::<Vec<_>>();
    let legacy_lock = prepare_exact_legacy_owner(
        record_matches,
        &parent,
        STATE_COMPONENTS[3],
        &ignored,
        binding,
        accepted,
        mode,
    )?;
    let mut fresh = match host_state::stage_quarantine_state(home, &parent, parent_lock, accepted) {
        Ok(fresh) => fresh,
        Err(error) => {
            // Staging may have already published a directory or its plan-bound
            // pending record. Only project the underlying no-effect error when
            // the stage is provably absent; otherwise preserve the observed
            // transition and report an effectful HOLD.
            return match parent.stat(BOOTSTRAP_STAGE) {
                Ok(None) => Err(error),
                Ok(Some(_)) | Err(_) => Ok(ambiguous_outcome(accepted, false)),
            };
        }
    };
    match parent.exchange_children(
        STATE_COMPONENTS[3],
        BOOTSTRAP_STAGE,
        ExclusivePublishSite::ExchangeLegacyAndFresh,
    ) {
        Ok(()) => {}
        Err(ExclusivePublishFailure::BeforeRename) => {
            return Ok(ambiguous_outcome(accepted, false));
        }
        Err(ExclusivePublishFailure::AfterRenameDurabilityUnknown) => {
            return Ok(ambiguous_outcome(accepted, false));
        }
    }
    host_state::rebind_state_owner(&mut fresh, &parent, STATE_COMPONENTS[3]);
    finish_committed_transition(parent, fresh, legacy_lock, accepted, true, false)
}

#[allow(clippy::too_many_arguments)]
fn apply_from_fresh_source(
    record_matches: &impl Fn(&RoutineStateMigrationAdmission) -> bool,
    home: AnchoredDirectory,
    parent: AnchoredDirectory,
    parent_lock: ParentStateLock,
    source: AnchoredDirectory,
    binding: CheckpointBinding<'_>,
    accepted: &RoutineStateQuarantinePlan,
    quarantine_present: bool,
    bootstrap_present: bool,
    mode: AdmissionMode,
) -> Result<RoutineStateQuarantineApplyOutcome, HostFailure> {
    let settlement = quarantine_transition::classify(&source, accepted)?;
    if bootstrap_present == quarantine_present
        || (settlement == quarantine_transition::SettlementState::None && bootstrap_present)
        || (settlement == quarantine_transition::SettlementState::Settled && bootstrap_present)
    {
        return Err(HostFailure::Invalid);
    }
    let observed_legacy = if bootstrap_present {
        BOOTSTRAP_STAGE
    } else if quarantine_present {
        accepted.quarantine_owner.as_str()
    } else {
        return Err(HostFailure::Invalid);
    };
    let legacy_lock = prepare_exact_legacy_owner(
        record_matches,
        &parent,
        observed_legacy,
        &[STATE_COMPONENTS[3]],
        binding,
        accepted,
        mode,
    )?;
    if settlement == quarantine_transition::SettlementState::Settled {
        verify_exact_digests(
            &parent,
            &accepted.quarantine_owner,
            &[STATE_COMPONENTS[3]],
            accepted,
        )?;
        let current = host_state::open_existing_state(home, source, parent_lock)?;
        current.verify()?;
        let settlement = quarantine_transition::settle(&current.state, accepted);
        drop(legacy_lock);
        return Ok(match settlement {
            quarantine_transition::SettlementPublication::DurableSettled => {
                RoutineStateQuarantineApplyOutcome {
                    status: "already_applied_no_effect",
                    effect: "none",
                    settlement_state: "settled_receipt_verified",
                    plan_id: accepted.plan_id.clone(),
                    quarantine_owner: accepted.quarantine_owner.clone(),
                    fresh_format_verified: true,
                }
            }
            quarantine_transition::SettlementPublication::ReceiptVisibleDurabilityUnacknowledged => {
                receipt_durability_unacknowledged_outcome(accepted)
            }
            quarantine_transition::SettlementPublication::StillPending
            | quarantine_transition::SettlementPublication::Invalid => {
                ambiguous_outcome(accepted, true)
            }
        });
    }
    if settlement == quarantine_transition::SettlementState::None {
        return Err(HostFailure::Invalid);
    }
    let fresh = host_state::open_quarantine_transition_state(
        home,
        &parent,
        STATE_COMPONENTS[3],
        parent_lock,
        accepted,
    )?;
    finish_committed_transition(
        parent,
        fresh,
        legacy_lock,
        accepted,
        bootstrap_present,
        true,
    )
}

fn prepare_exact_legacy_owner(
    record_matches: &impl Fn(&RoutineStateMigrationAdmission) -> bool,
    parent: &AnchoredDirectory,
    observed_owner: &str,
    ignored_parent_entries: &[&str],
    binding: CheckpointBinding<'_>,
    accepted: &RoutineStateQuarantinePlan,
    mode: AdmissionMode,
) -> Result<ProcessLock, HostFailure> {
    let state = parent.open_child(observed_owner)?;
    let admission = match mode {
        AdmissionMode::Migration => migration_admission::assess_format_absent(
            parent,
            state,
            binding,
            observed_owner,
            ignored_parent_entries,
        )?,
        AdmissionMode::ExplicitAbandonment => {
            migration_admission::assess_format_absent_for_abandonment(
                parent,
                state,
                binding,
                observed_owner,
                ignored_parent_entries,
            )?
        }
    };
    if admission.quarantine_plan.as_ref() != Some(accepted) || !record_matches(&admission) {
        return Err(HostFailure::Invalid);
    }

    // Retain the legacy adapter lock across the final digest check and whole
    // owner rename. This closes the compatibility window with older writers
    // that do not yet participate in the parent-directory lock.
    let state = parent.open_child(observed_owner)?;
    let adapter = state.open_child(ADAPTER_DIRECTORY)?;
    let legacy_lock =
        ProcessLock::acquire(adapter.open_regular(LOCK_NAME, libc::O_RDWR, 0o600)?)?;
    if host_state::read_lock_marker(&legacy_lock.0)? != LOCK_MARKER {
        return Err(HostFailure::Invalid);
    }
    verify_exact_digests(parent, observed_owner, ignored_parent_entries, accepted)?;
    Ok(legacy_lock)
}

fn verify_exact_digests(
    parent: &AnchoredDirectory,
    observed_owner: &str,
    ignored_parent_entries: &[&str],
    accepted: &RoutineStateQuarantinePlan,
) -> Result<(), HostFailure> {
    let state = parent.open_child(observed_owner)?;
    let (inventory, owner_tree) = migration_admission::observed_inventory_digests(
        parent,
        &state,
        observed_owner,
        ignored_parent_entries,
    )?;
    if inventory != accepted.source_inventory_sha256
        || owner_tree != accepted.source_owner_tree_sha256
    {
        return Err(HostFailure::Invalid);
    }
    Ok(())
}

fn finish_committed_transition(
    parent: AnchoredDirectory,
    fresh: HostState,
    legacy_lock: ProcessLock,
    accepted: &RoutineStateQuarantinePlan,
    legacy_at_bootstrap_stage: bool,
    recovered: bool,
) -> Result<RoutineStateQuarantineApplyOutcome, HostFailure> {
    if legacy_at_bootstrap_stage {
        match parent.publish_child_exclusive(
            BOOTSTRAP_STAGE,
            &accepted.quarantine_owner,
            ExclusivePublishSite::QuarantineSwappedLegacy,
        ) {
            Ok(()) => {}
            Err(_) => return Ok(ambiguous_outcome(accepted, false)),
        }
    } else if parent.file.sync_all().is_err() {
        // A prior finalize may have reached the namespace but returned before
        // its parent-directory durability was known. Exact replay must close
        // that barrier before it can publish the terminal settlement receipt.
        return Ok(ambiguous_outcome(accepted, false));
    }
    if verify_exact_digests(
        &parent,
        &accepted.quarantine_owner,
        &[STATE_COMPONENTS[3]],
        accepted,
    )
    .is_err()
        || host_state::verify_quarantine_transition_state(&fresh, accepted).is_err()
    {
        return Ok(ambiguous_outcome(accepted, false));
    }
    let Ok(quarantine) = parent.open_child(&accepted.quarantine_owner) else {
        return Ok(ambiguous_outcome(accepted, false));
    };
    let verified_quarantine = migration_admission::observed_owner_tree_sha256(
        &parent,
        &quarantine,
        &accepted.quarantine_owner,
        &[STATE_COMPONENTS[3]],
    );
    if verified_quarantine.as_deref().ok() != Some(accepted.source_owner_tree_sha256.as_str()) {
        return Ok(ambiguous_outcome(accepted, false));
    }
    drop(legacy_lock);
    match quarantine_transition::settle(&fresh.state, accepted) {
        quarantine_transition::SettlementPublication::DurableSettled => {
            if fresh.verify().is_err() {
                return Ok(settled_receipt_post_verify_hold(accepted));
            }
            Ok(RoutineStateQuarantineApplyOutcome {
                status: if recovered {
                    "recovered_then_applied"
                } else {
                    "applied"
                },
                effect: "staged_fresh_v8_then_atomically_exchanged_quarantined_legacy_and_published_settlement_receipt",
                settlement_state: "settled_receipt_durable",
                plan_id: accepted.plan_id.clone(),
                quarantine_owner: accepted.quarantine_owner.clone(),
                fresh_format_verified: true,
            })
        }
        quarantine_transition::SettlementPublication::ReceiptVisibleDurabilityUnacknowledged => {
            Ok(receipt_durability_unacknowledged_outcome(accepted))
        }
        quarantine_transition::SettlementPublication::StillPending
        | quarantine_transition::SettlementPublication::Invalid => {
            Ok(ambiguous_outcome(accepted, true))
        }
    }
}

fn ambiguous_outcome(
    accepted: &RoutineStateQuarantinePlan,
    fresh_format_verified: bool,
) -> RoutineStateQuarantineApplyOutcome {
    RoutineStateQuarantineApplyOutcome {
        status: "ambiguous_hold",
        effect: "host_state_namespace_or_durability_transition_preserved",
        settlement_state: "pending_transition_preserved",
        plan_id: accepted.plan_id.clone(),
        quarantine_owner: accepted.quarantine_owner.clone(),
        fresh_format_verified,
    }
}

fn receipt_durability_unacknowledged_outcome(
    accepted: &RoutineStateQuarantinePlan,
) -> RoutineStateQuarantineApplyOutcome {
    RoutineStateQuarantineApplyOutcome {
        status: "settlement_receipt_observed_durability_unacknowledged",
        effect: "host_state_migration_committed_and_settlement_receipt_visible_durability_unacknowledged",
        settlement_state: "settled_receipt_observed_durability_unacknowledged",
        plan_id: accepted.plan_id.clone(),
        quarantine_owner: accepted.quarantine_owner.clone(),
        fresh_format_verified: true,
    }
}

fn settled_receipt_post_verify_hold(
    accepted: &RoutineStateQuarantinePlan,
) -> RoutineStateQuarantineApplyOutcome {
    RoutineStateQuarantineApplyOutcome {
        status: "settlement_receipt_durable_post_verify_hold",
        effect: "host_state_migration_committed_and_settlement_receipt_durable_but_post_publication_verification_unavailable",
        settlement_state: "settled_receipt_durable_post_verify_unavailable",
        plan_id: accepted.plan_id.clone(),
        quarantine_owner: accepted.quarantine_owner.clone(),
        fresh_format_verified: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
        home: PathBuf,
        target: PathBuf,
        parent: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let root = fs::canonicalize(std::env::temp_dir())
                .unwrap()
                .join(format!(
                    "hul-quarantine-apply-{name}-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
            let home = root.join("home");
            let target = root.join("target");
            let parent = home.join(".codex/state/harness-ultragoal");
            let state = parent.join(STATE_COMPONENTS[3]);
            for directory in [
                &root,
                &home,
                &target,
                &home.join(".codex"),
                &home.join(".codex/state"),
                &parent,
                &state,
                &state.join(AUTHORITY_DIRECTORY),
                &state.join(ADAPTER_DIRECTORY),
                &state.join(LAUNCH_DIRECTORY),
            ] {
                fs::create_dir(directory).unwrap();
                fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
            }
            let lock = state.join(ADAPTER_DIRECTORY).join(LOCK_NAME);
            fs::write(&lock, LOCK_MARKER).unwrap();
            fs::set_permissions(&lock, fs::Permissions::from_mode(0o600)).unwrap();
            Self {
                root,
                home,
                target,
                parent,
            }
        }

        fn binding(&self) -> CheckpointBinding<'_> {
            CheckpointBinding::new(
                &self.target,
                "sha256:context",
                "sha256:candidate",
                "sha256:plan",
                "sha256:snapshot",
                "sha256:execution",
            )
        }

        fn plan(&self) -> RoutineStateQuarantinePlan {
            migration_admission::assess_migration_admission(&self.home, self.binding())
                .unwrap()
                .unwrap()
                .quarantine_plan
                .unwrap()
        }

        fn remove(self) {
            fs::remove_dir_all(self.root).unwrap();
        }
    }

    #[test]
    fn exchange_post_rename_durability_failure_never_leaves_the_active_owner_absent() {
        let fixture = Fixture::new("exchange-fsync");
        let plan = fixture.plan();
        fail_after_next_rename(ExclusivePublishSite::ExchangeLegacyAndFresh);

        let outcome = apply_quarantine_plan(
            &fixture.home,
            &fixture.target,
            fixture.binding(),
            &plan,
            &|admission| admission.quarantine_plan.as_ref() == Some(&plan),
        )
        .unwrap();
        assert_eq!(outcome.status, "ambiguous_hold");
        assert!(fixture.parent.join(STATE_COMPONENTS[3]).is_dir());
        assert!(fixture.parent.join(BOOTSTRAP_STAGE).is_dir());
        assert!(!fixture.parent.join(&plan.quarantine_owner).exists());
        assert!(
            fixture
                .parent
                .join(STATE_COMPONENTS[3])
                .join(QUARANTINE_TRANSITION_MARKER)
                .is_file()
        );

        let replay = apply_quarantine_plan(
            &fixture.home,
            &fixture.target,
            fixture.binding(),
            &plan,
            &|admission| admission.quarantine_plan.as_ref() == Some(&plan),
        )
        .unwrap();
        assert_eq!(replay.status, "recovered_then_applied");
        assert!(!fixture.parent.join(BOOTSTRAP_STAGE).exists());
        assert!(fixture.parent.join(&plan.quarantine_owner).is_dir());
        assert!(
            fixture
                .parent
                .join(STATE_COMPONENTS[3])
                .join(QUARANTINE_TRANSITION_MARKER)
                .is_file()
        );
        assert!(
            fixture
                .parent
                .join(STATE_COMPONENTS[3])
                .join(QUARANTINE_SETTLEMENT_RECEIPT)
                .is_file()
        );

        fixture.remove();
    }

    #[test]
    fn quarantine_finalize_durability_failure_preserves_both_owners_and_holds() {
        let fixture = Fixture::new("finalize-fsync");
        let plan = fixture.plan();
        fail_after_next_rename(ExclusivePublishSite::QuarantineSwappedLegacy);

        let outcome = apply_quarantine_plan(
            &fixture.home,
            &fixture.target,
            fixture.binding(),
            &plan,
            &|admission| admission.quarantine_plan.as_ref() == Some(&plan),
        )
        .unwrap();
        assert_eq!(outcome.status, "ambiguous_hold");
        assert!(fixture.parent.join(STATE_COMPONENTS[3]).is_dir());
        assert!(fixture.parent.join(&plan.quarantine_owner).is_dir());
        assert!(!fixture.parent.join(BOOTSTRAP_STAGE).exists());
        assert!(
            fixture
                .parent
                .join(STATE_COMPONENTS[3])
                .join(QUARANTINE_TRANSITION_MARKER)
                .is_file()
        );

        let replay = apply_quarantine_plan(
            &fixture.home,
            &fixture.target,
            fixture.binding(),
            &plan,
            &|admission| admission.quarantine_plan.as_ref() == Some(&plan),
        )
        .unwrap();
        assert_eq!(replay.status, "recovered_then_applied");
        assert!(
            fixture
                .parent
                .join(STATE_COMPONENTS[3])
                .join(QUARANTINE_TRANSITION_MARKER)
                .is_file()
        );
        assert!(
            fixture
                .parent
                .join(STATE_COMPONENTS[3])
                .join(QUARANTINE_SETTLEMENT_RECEIPT)
                .is_file()
        );

        fixture.remove();
    }

    #[test]
    fn visible_settlement_receipt_is_effectful_and_exact_replay_settles_no_effect() {
        let fixture = Fixture::new("settlement-receipt-fsync");
        let plan = fixture.plan();
        quarantine_transition::fail_after_next_receipt_rename();

        let outcome = apply_quarantine_plan(
            &fixture.home,
            &fixture.target,
            fixture.binding(),
            &plan,
            &|admission| admission.quarantine_plan.as_ref() == Some(&plan),
        )
        .unwrap();
        assert_eq!(
            outcome.status,
            "settlement_receipt_observed_durability_unacknowledged"
        );
        assert_ne!(outcome.effect, "none");
        assert!(fixture.parent.join(STATE_COMPONENTS[3]).is_dir());
        assert!(fixture.parent.join(&plan.quarantine_owner).is_dir());
        assert!(outcome.fresh_format_verified);
        assert!(
            fixture
                .parent
                .join(STATE_COMPONENTS[3])
                .join(QUARANTINE_TRANSITION_MARKER)
                .is_file()
        );
        assert!(
            fixture
                .parent
                .join(STATE_COMPONENTS[3])
                .join(QUARANTINE_SETTLEMENT_RECEIPT)
                .is_file()
        );
        HostState::open_existing_for_target(&fixture.home, &fixture.target).unwrap();

        quarantine_transition::fail_next_receipt_acknowledgement();
        let replay = apply_quarantine_plan(
            &fixture.home,
            &fixture.target,
            fixture.binding(),
            &plan,
            &|admission| admission.quarantine_plan.as_ref() == Some(&plan),
        )
        .unwrap();
        assert_eq!(
            replay.status,
            "settlement_receipt_observed_durability_unacknowledged"
        );
        assert_ne!(replay.effect, "none");

        let replay = apply_quarantine_plan(
            &fixture.home,
            &fixture.target,
            fixture.binding(),
            &plan,
            &|admission| admission.quarantine_plan.as_ref() == Some(&plan),
        )
        .unwrap();
        assert_eq!(replay.status, "already_applied_no_effect");
        assert_eq!(replay.effect, "none");
        assert_eq!(replay.settlement_state, "settled_receipt_verified");

        quarantine_transition::fail_next_settled_replay_evidence_check();
        let changed_evidence = apply_quarantine_plan(
            &fixture.home,
            &fixture.target,
            fixture.binding(),
            &plan,
            &|admission| admission.quarantine_plan.as_ref() == Some(&plan),
        )
        .unwrap();
        assert_eq!(changed_evidence.status, "ambiguous_hold");
        assert_ne!(changed_evidence.effect, "none");
        assert_eq!(
            changed_evidence.settlement_state,
            "pending_transition_preserved"
        );
        assert!(
            !fixture
                .parent
                .join(STATE_COMPONENTS[3])
                .join(QUARANTINE_TRANSITION_MARKER)
                .exists()
        );
        assert!(matches!(
            HostState::open_existing_for_target(&fixture.home, &fixture.target),
            Err(HostFailure::TransitionAmbiguous)
        ));

        fixture.remove();
    }

    #[test]
    fn staged_receipt_preserves_the_hold_until_exact_replay_publishes_it() {
        let fixture = Fixture::new("settlement-receipt-staged");
        let plan = fixture.plan();
        quarantine_transition::fail_before_next_receipt_publication();

        let outcome = apply_quarantine_plan(
            &fixture.home,
            &fixture.target,
            fixture.binding(),
            &plan,
            &|admission| admission.quarantine_plan.as_ref() == Some(&plan),
        )
        .unwrap();
        assert_eq!(outcome.status, "ambiguous_hold");
        assert_ne!(outcome.effect, "none");
        let state = fixture.parent.join(STATE_COMPONENTS[3]);
        assert!(state.join(QUARANTINE_TRANSITION_MARKER).is_file());
        assert!(state.join(QUARANTINE_SETTLEMENT_STAGE).is_file());
        assert!(!state.join(QUARANTINE_SETTLEMENT_RECEIPT).exists());
        assert!(matches!(
            HostState::open_existing_for_target(&fixture.home, &fixture.target),
            Err(HostFailure::TransitionAmbiguous)
        ));

        let replay = apply_quarantine_plan(
            &fixture.home,
            &fixture.target,
            fixture.binding(),
            &plan,
            &|admission| admission.quarantine_plan.as_ref() == Some(&plan),
        )
        .unwrap();
        assert_eq!(replay.status, "recovered_then_applied");
        assert!(state.join(QUARANTINE_TRANSITION_MARKER).is_file());
        assert!(state.join(QUARANTINE_SETTLEMENT_RECEIPT).is_file());
        assert!(!state.join(QUARANTINE_SETTLEMENT_STAGE).exists());
        HostState::open_existing_for_target(&fixture.home, &fixture.target).unwrap();

        fixture.remove();
    }

    #[test]
    fn pre_exchange_refusal_after_staging_reports_an_effectful_hold() {
        let fixture = Fixture::new("pre-exchange-refusal");
        let plan = fixture.plan();
        fail_before_next_rename(ExclusivePublishSite::ExchangeLegacyAndFresh);

        let outcome = apply_quarantine_plan(
            &fixture.home,
            &fixture.target,
            fixture.binding(),
            &plan,
            &|admission| admission.quarantine_plan.as_ref() == Some(&plan),
        )
        .unwrap();
        assert_eq!(outcome.status, "ambiguous_hold");
        assert_ne!(outcome.effect, "none");
        assert!(fixture.parent.join(BOOTSTRAP_STAGE).is_dir());
        assert!(
            fixture
                .parent
                .join(BOOTSTRAP_STAGE)
                .join(QUARANTINE_TRANSITION_MARKER)
                .is_file()
        );
        assert!(matches!(
            HostState::open_or_bootstrap(&fixture.home, &fixture.target),
            Err(HostFailure::TransitionAmbiguous)
        ));

        fixture.remove();
    }

    #[test]
    fn staging_failure_after_the_pending_record_reports_an_effectful_hold() {
        let fixture = Fixture::new("staging-after-pending");
        let plan = fixture.plan();
        host_state::fail_after_next_quarantine_marker_begin();

        let outcome = apply_quarantine_plan(
            &fixture.home,
            &fixture.target,
            fixture.binding(),
            &plan,
            &|admission| admission.quarantine_plan.as_ref() == Some(&plan),
        )
        .unwrap();
        assert_eq!(outcome.status, "ambiguous_hold");
        assert_ne!(outcome.effect, "none");
        assert_eq!(outcome.settlement_state, "pending_transition_preserved");
        assert!(fixture.parent.join(BOOTSTRAP_STAGE).is_dir());
        assert!(
            fixture
                .parent
                .join(BOOTSTRAP_STAGE)
                .join(QUARANTINE_TRANSITION_MARKER)
                .is_file()
        );
        assert!(matches!(
            HostState::open_or_bootstrap(&fixture.home, &fixture.target),
            Err(HostFailure::TransitionAmbiguous)
        ));

        fixture.remove();
    }

    #[test]
    fn legacy_and_fresh_adapter_locks_cover_both_sides_of_the_atomic_exchange() {
        fn assert_lock_busy(path: &Path) {
            let file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .unwrap();
            assert!(matches!(ProcessLock::acquire(file), Err(HostFailure::Busy)));
        }

        let fixture = Fixture::new("two-version-lock-handoff");
        let plan = fixture.plan();
        let home = host_state::open_home(&fixture.home).unwrap();
        let codex = home.open_child(STATE_COMPONENTS[0]).unwrap();
        let state_root = codex.open_child(STATE_COMPONENTS[1]).unwrap();
        let parent = state_root.open_child(STATE_COMPONENTS[2]).unwrap();
        let parent_lock = ParentStateLock::exclusive(&parent).unwrap();
        let legacy_lock = prepare_exact_legacy_owner(
            &|admission| admission.quarantine_plan.as_ref() == Some(&plan),
            &parent,
            STATE_COMPONENTS[3],
            &[],
            fixture.binding(),
            &plan,
            AdmissionMode::Migration,
        )
        .unwrap();
        let fresh = host_state::stage_quarantine_state(home, &parent, parent_lock, &plan).unwrap();

        assert_lock_busy(
            &fixture
                .parent
                .join(STATE_COMPONENTS[3])
                .join(ADAPTER_DIRECTORY)
                .join(LOCK_NAME),
        );
        assert_lock_busy(
            &fixture
                .parent
                .join(BOOTSTRAP_STAGE)
                .join(ADAPTER_DIRECTORY)
                .join(LOCK_NAME),
        );
        parent
            .exchange_children(
                STATE_COMPONENTS[3],
                BOOTSTRAP_STAGE,
                ExclusivePublishSite::ExchangeLegacyAndFresh,
            )
            .unwrap();
        assert_lock_busy(
            &fixture
                .parent
                .join(STATE_COMPONENTS[3])
                .join(ADAPTER_DIRECTORY)
                .join(LOCK_NAME),
        );
        assert_lock_busy(
            &fixture
                .parent
                .join(BOOTSTRAP_STAGE)
                .join(ADAPTER_DIRECTORY)
                .join(LOCK_NAME),
        );

        drop(fresh);
        drop(legacy_lock);
        drop(parent);
        fixture.remove();
    }
}
