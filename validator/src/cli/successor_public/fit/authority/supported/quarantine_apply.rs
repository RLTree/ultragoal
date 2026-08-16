use super::super::{
    RepositoryFitAuthorityMigrationRecord, RepositoryFitAuthorityQuarantinePlan,
    record_matches_live,
};
use super::state_open::open_host_state_base;
use super::*;
use crate::cli::successor::ExitClass;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum QuarantineStatus {
    QuarantinedAndFreshOwnerBootstrapped,
    RolledBackBeforeBootstrap,
    AmbiguousAfterBootstrapStarted,
}

#[derive(Serialize)]
struct QuarantineOutcome<'a> {
    schema_version: &'static str,
    status: QuarantineStatus,
    plan_id: &'a str,
    source_inventory_sha256: &'a str,
    source_owner: &'a str,
    quarantine_owner: &'a str,
    target_format: &'a str,
    effect: &'static str,
    effect_started: bool,
    rollback_complete: bool,
    fresh_owner_bootstrapped: bool,
    quarantine_retained: bool,
    target_effect: &'static str,
    next_action: &'static str,
    claim_effect: &'static str,
    claim_ceiling: &'static str,
}

pub(crate) fn execute_quarantine(
    context: &LiveContext,
    home: &Path,
    record: &RepositoryFitAuthorityMigrationRecord,
    accepted_plan: &str,
) -> Result<RuntimeOutcome, HostFailure> {
    if context.revalidate().is_err()
        || record.quarantine_plan.plan_id != accepted_plan
        || !valid_digest(accepted_plan)
    {
        return Err(HostFailure::QuarantineStale);
    }
    let Some(base) = open_host_state_base(home)? else {
        return Err(HostFailure::Unavailable);
    };
    let Some(parent) = base.open_child_optional(FIT_STATE_COMPONENTS[0], true)? else {
        return Err(HostFailure::Unavailable);
    };
    let migration_lock = ParentStateLock::acquire_exclusive(&parent)?;
    migration_lock.verify(&parent)?;

    let live = assess_migration_admission(home)?
        .filter(|live| record_matches_live(record, live))
        .ok_or(HostFailure::QuarantineStale)?;
    let plan = live
        .quarantine_plan
        .as_ref()
        .ok_or(HostFailure::QuarantineStale)?;
    if parent
        .open_child_optional(&plan.quarantine_owner, true)?
        .is_some()
    {
        return Err(HostFailure::QuarantineStale);
    }
    parent.verify(true)?;
    context
        .revalidate()
        .map_err(|_| HostFailure::QuarantineStale)?;
    failpoint("before_rename")?;

    rename_exclusive(&parent.file, plan.source_owner, &plan.quarantine_owner)?;
    let mut bootstrap_started = false;
    let transition = (|| {
        failpoint("after_rename_before_parent_sync")?;
        sync_directory(&parent.file)?;
        parent.verify(true)?;
        failpoint("after_parent_sync_before_bootstrap")?;

        let quarantined = parent.open_child(&plan.quarantine_owner, true)?;
        let post_move = assess_migration_admission_for_owner(quarantined, parent.identity)?
            .filter(|observed| record_matches_live(record, observed))
            .ok_or(HostFailure::QuarantineStale)?;
        if post_move.quarantine_plan.as_ref() != Some(plan) {
            return Err(HostFailure::QuarantineStale);
        }

        bootstrap_started = true;
        let state = HostState::initialize_under_migration_lock(
            home,
            context.worktree_root(),
            &parent,
            &migration_lock,
        )?;
        failpoint("after_fresh_owner_created")?;
        let _ledger =
            FileRepositoryFitLedger::open_or_initialize(&state.store.root, state.store.store_id())
                .map_err(|_| HostFailure::Persistence)?;
        state.verify()?;
        parent.verify(true)?;
        migration_lock.verify(&parent)?;
        Ok(())
    })();

    match transition {
        Ok(()) => render_outcome(
            QuarantineStatus::QuarantinedAndFreshOwnerBootstrapped,
            plan,
            true,
            false,
            true,
            "rerun the exact target fit plan and accepted apply through the installed runtime",
        ),
        Err(_) if !bootstrap_started => {
            if rollback_before_bootstrap(&parent, plan, record).is_ok() {
                render_outcome(
                    QuarantineStatus::RolledBackBeforeBootstrap,
                    plan,
                    true,
                    true,
                    false,
                    "diagnose the unchanged authenticated owner before recomputing a quarantine plan",
                )
            } else {
                render_outcome(
                    QuarantineStatus::AmbiguousAfterBootstrapStarted,
                    plan,
                    true,
                    false,
                    false,
                    "preserve both owner names and diagnose the exact parent inventory before any retry",
                )
            }
        }
        Err(_) => render_outcome(
            QuarantineStatus::AmbiguousAfterBootstrapStarted,
            plan,
            true,
            false,
            false,
            "preserve the quarantine and fresh-owner bytes and diagnose them before any retry",
        ),
    }
}

fn rollback_before_bootstrap(
    parent: &AnchoredDirectory,
    plan: &RepositoryFitAuthorityQuarantinePlan,
    record: &RepositoryFitAuthorityMigrationRecord,
) -> Result<(), HostFailure> {
    if parent
        .open_child_optional(plan.source_owner, true)?
        .is_some()
    {
        return Err(HostFailure::Persistence);
    }
    rename_exclusive(&parent.file, &plan.quarantine_owner, plan.source_owner)?;
    sync_directory(&parent.file)?;
    let restored = parent.open_child(plan.source_owner, true)?;
    let live = assess_migration_admission_for_owner(restored, parent.identity)?
        .filter(|live| record_matches_live(record, live))
        .ok_or(HostFailure::Persistence)?;
    if live.quarantine_plan.as_ref() != Some(plan) {
        return Err(HostFailure::Persistence);
    }
    Ok(())
}

fn render_outcome(
    status: QuarantineStatus,
    plan: &RepositoryFitAuthorityQuarantinePlan,
    effect_started: bool,
    rollback_complete: bool,
    fresh_owner_bootstrapped: bool,
    next_action: &'static str,
) -> Result<RuntimeOutcome, HostFailure> {
    let machine = serde_json::to_vec(&QuarantineOutcome {
        schema_version: "RepositoryFitAuthorityQuarantineOutcome-v1",
        status,
        plan_id: &plan.plan_id,
        source_inventory_sha256: &plan.source_inventory_sha256,
        source_owner: plan.source_owner,
        quarantine_owner: &plan.quarantine_owner,
        target_format: plan.target_format,
        effect: "host_state_write",
        effect_started,
        rollback_complete,
        fresh_owner_bootstrapped,
        quarantine_retained: !rollback_complete,
        target_effect: "none",
        next_action,
        claim_effect: "none",
        claim_ceiling: "repository-fit authority quarantine transition only; target fit, installed journey, product, readiness, release, and completion remain unproved",
    })
    .map_err(|_| HostFailure::Persistence)?;
    let class = if status == QuarantineStatus::QuarantinedAndFreshOwnerBootstrapped {
        ExitClass::Success
    } else {
        ExitClass::ActionableFinding
    };
    Ok(RuntimeOutcome::payload(
        class,
        machine,
        format!(
            "repository-fit authority quarantine status={status:?} effect=host_state_write target_effect=none"
        ),
    ))
}

#[cfg(test)]
thread_local! {
    static QUARANTINE_FAILPOINT: std::cell::RefCell<Option<&'static str>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(crate) fn set_quarantine_failpoint_for_test(value: Option<&'static str>) {
    QUARANTINE_FAILPOINT.with(|slot| *slot.borrow_mut() = value);
}

fn failpoint(name: &'static str) -> Result<(), HostFailure> {
    #[cfg(test)]
    if QUARANTINE_FAILPOINT.with(|slot| slot.borrow().as_ref() == Some(&name)) {
        return Err(HostFailure::Persistence);
    }
    let _ = name;
    Ok(())
}
