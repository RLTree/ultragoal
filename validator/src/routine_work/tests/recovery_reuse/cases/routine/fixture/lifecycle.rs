use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};

use super::base_support::TempRepo;
use super::routine_fixture_inventory::RoutineFixtureInventory;
use super::routine_fixture_invocation::run_routine_fixture_invocation;
use super::routine_fixture_workspace::RoutineFixtureTeardown;

#[test]
fn assertion_unwind_is_reclaimed_by_the_invocation_owner() {
    let inventory = RoutineFixtureInventory::capture(&["journey-assertion-unwind"]);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        run_routine_fixture_invocation(&["journey-assertion-unwind"], |repositories| {
            let [repo] = repositories else {
                panic!("one assertion repository is required")
            };
            repo.write("src/lib.rs", b"failure before fixture teardown\n");
            panic!("induced assertion interruption");
        });
    }));
    assert!(outcome.is_err());
    inventory.assert_unchanged();
}

#[test]
fn multi_repository_assertion_unwind_reclaims_every_claim() {
    let labels = [
        "journey-multi-clean",
        "journey-multi-dirty",
        "journey-multi-conflict",
        "journey-multi-fallback",
    ];
    let inventory = RoutineFixtureInventory::capture(&labels);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        run_routine_fixture_invocation(&labels, |repositories| {
            assert_eq!(repositories.len(), labels.len());
            for repo in repositories {
                repo.write("failure-state.txt", b"owned failure state\n");
            }
            panic!("induced multi-repository interruption");
        });
    }));
    assert!(outcome.is_err());
    inventory.assert_unchanged();
}

#[test]
fn initialization_failure_rolls_back_the_exact_claim() {
    let inventory = RoutineFixtureInventory::capture(&["journey-initialization-failure"]);
    let failure = match TempRepo::interrupted_initialization("journey-initialization-failure") {
        Err(failure) => failure,
        Ok(_) => panic!("interrupted initialization must fail"),
    };
    assert_eq!(failure.id(), "routine-fixture-initialization-interrupted");
    inventory.assert_unchanged();
}

#[test]
fn concurrent_claims_are_unique_and_leave_no_invocation_residue() {
    const LABEL: &str = "journey-concurrent-claim";
    let inventory = RoutineFixtureInventory::capture(&[LABEL]);
    let actors = (0..8)
        .map(|index| {
            std::thread::spawn(move || {
                run_routine_fixture_invocation(&[LABEL], |repositories| {
                    let [repo] = repositories else {
                        panic!("one concurrent repository is required")
                    };
                    assert!(repo.status().is_empty());
                    if index == 3 {
                        panic!("induced concurrent assertion interruption");
                    }
                });
            })
        })
        .collect::<Vec<_>>();
    let outcomes = actors
        .into_iter()
        .map(|actor| actor.join())
        .collect::<Vec<_>>();
    assert_eq!(
        outcomes
            .into_iter()
            .filter(|outcome| outcome.is_err())
            .count(),
        1
    );
    inventory.assert_unchanged();
}

#[cfg(target_os = "macos")]
#[test]
fn quiescent_substitution_removes_owned_repo_and_preserves_replacement() {
    const LABEL: &str = "journey-substituted-claim";
    let inventory = RoutineFixtureInventory::capture(&[LABEL]);
    run_routine_fixture_invocation(&[LABEL], |repositories| {
        let [repo] = repositories else {
            panic!("one substituted repository is required")
        };
        let original = repo.root().to_path_buf();
        let held = original.with_extension("owned-held");
        fs::rename(&original, &held).unwrap();
        fs::create_dir(&original).unwrap();
        let replacement = original.join("replacement.txt");
        fs::write(&replacement, b"unrelated replacement\n").unwrap();

        let outcome = repo.try_teardown_after_assertions();
        assert_eq!(
            outcome,
            RoutineFixtureTeardown::RemovedAfterSubstitution {
                owned_path: held.clone()
            }
        );
        assert!(!held.exists());
        assert_eq!(fs::read(&replacement).unwrap(), b"unrelated replacement\n");

        fs::remove_dir_all(&original).unwrap();
    });
    inventory.assert_unchanged();
}
