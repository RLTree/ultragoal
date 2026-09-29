use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

use super::base_support::TempRepo;
use super::routine_fixture_inventory::RoutineFixtureInventory;
use super::routine_fixture_workspace::RoutineFixtureOwner;

pub(crate) fn run_routine_fixture_invocation(labels: &[&str], body: impl FnOnce(&mut [TempRepo])) {
    let owner = RoutineFixtureOwner::new();
    let inventory = RoutineFixtureInventory::capture_owner(&owner);
    let mut repositories = Vec::with_capacity(labels.len());
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        for label in labels {
            repositories.push(TempRepo::new_in_invocation(label, &owner));
        }
        body(&mut repositories);
    }));

    let teardown = repositories
        .iter_mut()
        .rev()
        .map(TempRepo::try_teardown_after_assertions)
        .collect::<Vec<_>>();
    for result in teardown {
        result.assert_removed();
    }
    inventory.assert_unchanged();

    if let Err(payload) = outcome {
        resume_unwind(payload);
    }
}
