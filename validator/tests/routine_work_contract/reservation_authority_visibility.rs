use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use super::owned_compile_scratch::OwnedCompileScratch;

#[path = "reservation_authority_shape.rs"]
mod reservation_authority_shape;

const CLOSED: &str = r#"
mod mediator {
    pub(crate) mod reservation_state {
        mod authority {
            mod owner {
                mod attempt {
                    use std::cell::{Cell, RefCell};
                    struct AttemptReservation {
                        started: Cell<bool>, settled: Cell<bool>, staged: RefCell<Vec<()>>,
                    }
                    pub(in crate::mediator) struct ReservationAttempt<'a> {
                        owner: &'a AttemptReservation,
                    }
                    impl ReservationAttempt<'_> {
                        pub(in crate::mediator) fn mark_started(&self) { self.owner.started.set(true); }
                    }
                }
                mod registry {
                    struct RegistryState { active: Vec<()> }
                    fn raw() -> RegistryState { RegistryState { active: Vec::new() } }
                    pub(super) fn reserve() {}
                    pub fn registry_sibling_raw(_: super::attempt::AttemptReservation) {} // registry_sibling_raw
                }
                pub(in crate::mediator) use attempt::ReservationAttempt;
            }
            pub(in crate::mediator) use owner::ReservationAttempt;
            mod authority_sibling_attack {
                pub fn authority_sibling_raw(_: super::owner::attempt::AttemptReservation) {} // authority_sibling_raw
                pub fn authority_sibling_registry() { super::owner::registry::reserve(); } // authority_sibling_registry
            }
        }
        pub(super) use authority::ReservationAttempt;
        pub mod reservation_state_sibling_attack {
            use super::ReservationAttempt;
            pub fn reservation_state_sibling_owner(a: &ReservationAttempt<'_>) {
                a.owner.started.set(true); // reservation_state_sibling_owner
            }
            pub fn reservation_state_sibling_terminal(a: &ReservationAttempt<'_>) {
                a.settle_terminal(); // reservation_state_sibling_terminal
            }
        }
    }
    pub mod mediator_sibling_attack {
        use super::reservation_state::ReservationAttempt;
        pub fn mediator_sibling_terminal(a: &ReservationAttempt<'_>) { a.settle_terminal(); } // mediator_sibling_terminal
    }
}
mod crate_sibling_attack {
    pub fn crate_sibling_registry() {
        drop(crate::mediator::reservation_state::authority::owner::registry::raw()); // crate_sibling_registry
    }
}
"#;

const OPENED: &str = r#"
mod owner {
    use std::cell::{Cell, RefCell};
    pub(crate) struct AttemptReservation {
        pub(crate) started: Cell<bool>,
        pub(crate) settled: Cell<bool>,
        pub(crate) staged: RefCell<Vec<()>>,
    }
    pub(crate) struct ReservationAttempt<'a> { pub(crate) owner: &'a AttemptReservation }
    impl ReservationAttempt<'_> {
        pub(crate) fn settle_terminal(&self) { self.owner.settled.set(true); }
    }
    pub(crate) fn fresh() -> AttemptReservation {
        AttemptReservation {
            started: Cell::new(false), settled: Cell::new(false),
            staged: RefCell::new(vec![()]),
        }
    }
}
mod attacks {
    use super::owner::{AttemptReservation, ReservationAttempt};
    pub fn raw(owner: &AttemptReservation) {
        owner.started.set(true); owner.staged.borrow_mut().clear();
    }
    pub fn terminal(owner: &AttemptReservation) {
        ReservationAttempt { owner }.settle_terminal();
    }
}
#[test]
fn opened_attacks_execute() {
    let owner = owner::fresh();
    attacks::raw(&owner);
    attacks::terminal(&owner);
    assert!(owner.started.get() && owner.settled.get() && owner.staged.borrow().is_empty());
}
"#;

#[test]
fn closest_siblings_cannot_name_raw_or_terminal_authority() {
    let mut owned = OwnedCompileScratch::claim("routine-reservation-privacy");
    prepare(owned.path());
    fs::write(owned.path().join("lib.rs"), OPENED).unwrap();
    let opened = cargo(owned.path(), &["test", "--offline", "--quiet"]);
    assert!(opened.status.success(), "{}", diagnostic(&opened));

    fs::write(owned.path().join("lib.rs"), CLOSED).unwrap();
    let closed = cargo(owned.path(), &["check", "--offline", "--quiet"]);
    assert!(
        !closed.status.success(),
        "closed authority attacks compiled"
    );
    let stderr = diagnostic(&closed);
    for marker in [
        "authority_sibling_raw",
        "authority_sibling_registry",
        "registry_sibling_raw",
        "reservation_state_sibling_owner",
        "reservation_state_sibling_terminal",
        "mediator_sibling_terminal",
        "crate_sibling_registry",
    ] {
        assert!(stderr.contains(marker), "missing {marker}: {stderr}");
    }
    assert!(stderr.contains("is private"));
    assert!(stderr.contains("no method named `settle_terminal`"));
    owned.teardown_after_assertions();
}

#[test]
fn childless_owner_and_exact_paths_reject_causal_mutants() {
    let authority = include_str!(
        "../../src/routine_work/runtime_adapter/mediator/reservation_state/authority.rs"
    );
    let owner = include_str!(
        "../../src/routine_work/runtime_adapter/mediator/reservation_state/authority/owner.rs"
    );
    let attempt = include_str!(
        "../../src/routine_work/runtime_adapter/mediator/reservation_state/authority/owner/attempt.rs"
    );
    let registry = include_str!(
        "../../src/routine_work/runtime_adapter/mediator/reservation_state/authority/owner/registry.rs"
    );
    let state =
        include_str!("../../src/routine_work/runtime_adapter/mediator/reservation_state/mod.rs");
    let mediator = include_str!("../../src/routine_work/runtime_adapter/mediator/mod.rs");
    assert_eq!(
        reservation_authority_shape::validate(authority, owner, attempt, registry, state, mediator,),
        Ok(())
    );
    let descendant = format!(
        "{attempt}\nmod descendant_escape {{ pub fn mutate(a: &super::AttemptReservation) {{ a.started.set(true); }} }}\n"
    );
    assert_eq!(
        reservation_authority_shape::validate(
            authority,
            owner,
            &descendant,
            registry,
            state,
            mediator,
        ),
        Err("owner-leaf")
    );
    let redirected = authority.replace("authority/owner.rs", "authority/decoy.rs");
    assert_eq!(
        reservation_authority_shape::validate(
            &redirected,
            owner,
            attempt,
            registry,
            state,
            mediator,
        ),
        Err("authority-module-paths")
    );
    let wrapper_redirect = owner.replace("owner/attempt.rs", "owner/decoy.rs");
    assert_eq!(
        reservation_authority_shape::validate(
            authority,
            &wrapper_redirect,
            attempt,
            registry,
            state,
            mediator,
        ),
        Err("owner-module-paths")
    );
    let terminal = attempt.replace(
        "impl ReservationAttempt<'_> {",
        "impl ReservationAttempt<'_> {\n    pub(in super::super::super::super) fn settle_incomplete(&self) {}",
    );
    assert_eq!(
        reservation_authority_shape::validate(
            authority, owner, &terminal, registry, state, mediator,
        ),
        Err("owner-capability-methods")
    );
    let registry_mutator = format!("{registry}\npub(super) fn clear_all() {{}}\n");
    assert_eq!(
        reservation_authority_shape::validate(
            authority,
            owner,
            attempt,
            &registry_mutator,
            state,
            mediator,
        ),
        Err("registry-command-surface")
    );
}

fn prepare(scratch: &Path) {
    fs::write(
        scratch.join("Cargo.toml"),
        "[workspace]\n\n[package]\nname = \"reservation-authority-visibility\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lib]\npath = \"lib.rs\"\n",
    )
    .unwrap();
}

fn cargo(scratch: &Path, arguments: &[&str]) -> Output {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(arguments)
        .current_dir(scratch)
        .env("CARGO_TARGET_DIR", scratch.join("target"))
        .output()
        .unwrap()
}

fn diagnostic(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
