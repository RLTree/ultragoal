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
            mod transition_flag {
                use std::cell::Cell;
                pub(super) struct TransitionFlag(Cell<bool>);
                impl TransitionFlag {
                    pub(super) fn new() -> Self { Self(Cell::new(false)) }
                    pub(super) fn is_set(&self) -> bool { self.0.get() }
                    pub(super) fn mark(&self) { self.0.set(true); }
                }
            }
            mod staged_custody {
                use std::cell::RefCell;
                pub(super) struct StagedCustody(RefCell<Vec<()>>);
                impl StagedCustody {
                    pub(super) fn new() -> Self { Self(RefCell::new(vec![()])) }
                    pub(super) fn is_empty(&self) -> bool { self.0.borrow().is_empty() }
                    pub(super) fn clear_recorded(&self) { self.0.borrow_mut().clear(); }
                }
            }
            mod custody {
                use super::staged_custody::StagedCustody;
                use super::transition_flag::TransitionFlag;
                pub(super) struct AttemptCustody {
                    started: TransitionFlag,
                    settled: TransitionFlag,
                    staged: StagedCustody,
                }
                impl AttemptCustody {
                    pub(super) fn new() -> Self {
                        Self {
                            started: TransitionFlag::new(),
                            settled: TransitionFlag::new(),
                            staged: StagedCustody::new(),
                        }
                    }
                }
            }
            use custody::AttemptCustody;
            pub(in crate::mediator) struct AttemptReservation { custody: AttemptCustody }
            pub(in crate::mediator) fn fresh() -> AttemptReservation {
                AttemptReservation { custody: AttemptCustody::new() }
            }
        }
        pub(in crate::mediator) use authority::{AttemptReservation, fresh};
        pub mod descendant_attack {
            use super::AttemptReservation;
            pub fn started(a: &AttemptReservation) { a.custody.started.mark(); }
            pub fn settled(a: &AttemptReservation) { a.custody.settled.mark(); }
            pub async fn staged(a: &AttemptReservation) { a.custody.staged.clear_recorded(); }
        }
    }
    pub mod sibling_attack {
        use super::reservation_state::AttemptReservation;
        pub fn started(a: &AttemptReservation) { a.custody.started.mark(); }
        pub fn settled(a: &AttemptReservation) { a.custody.settled.mark(); }
        pub fn staged(a: &AttemptReservation) { a.custody.staged.clear_recorded(); }
    }
    // OPENED_EXTENSION
}
"#;

#[test]
fn sibling_and_descendant_cannot_mutate_reservation_storage() {
    reservation_authority_shape::assert_current();
    let mut owned = OwnedCompileScratch::claim("routine-reservation-privacy");
    prepare(owned.path());
    fs::write(owned.path().join("lib.rs"), opened()).unwrap();
    let control = run(owned.path());
    assert!(control.status.success(), "{}", diagnostic(&control));

    fs::write(owned.path().join("lib.rs"), CLOSED).unwrap();
    let closed = check(owned.path());
    assert!(!closed.status.success(), "closed mutation probe compiled");
    let stderr = String::from_utf8_lossy(&closed.stderr);
    assert!(stderr.matches("error[E0616]").count() >= 6, "{stderr}");
    assert!(stderr.matches("field `custody`").count() >= 6, "{stderr}");
    owned.teardown_after_assertions();
}

#[test]
fn reservation_owner_leaves_reject_new_mutation_surfaces() {
    let authority = include_str!(
        "../../src/routine_work/runtime_adapter/mediator/reservation_state/authority.rs"
    );
    let custody = include_str!(
        "../../src/routine_work/runtime_adapter/mediator/reservation_state/authority/custody.rs"
    );
    let staged = include_str!(
        "../../src/routine_work/runtime_adapter/mediator/reservation_state/authority/staged_custody.rs"
    );
    let flag = include_str!(
        "../../src/routine_work/runtime_adapter/mediator/reservation_state/authority/transition_flag.rs"
    );
    let method = "    pub(super) fn is_started(&self) -> bool {";
    let attributed = custody.replacen(
        method,
        "    #[cfg(test)]\n    pub(super) fn is_started(&self) -> bool {",
        1,
    );
    assert_eq!(
        reservation_authority_shape::validate(authority, &attributed, staged, flag),
        Err("authority-method-signature")
    );
    let extra = custody.replacen(
        method,
        "    pub(super) fn replace_started(&self) {}\n\n    pub(super) fn is_started(&self) -> bool {",
        1,
    );
    assert_eq!(
        reservation_authority_shape::validate(authority, &extra, staged, flag),
        Err("authority-methods")
    );
    let visible = custody.replacen(
        "    started: TransitionFlag,",
        "    pub(super) started: TransitionFlag,",
        1,
    );
    assert_eq!(
        reservation_authority_shape::validate(authority, &visible, staged, flag),
        Err("authority-field-visible")
    );
    let nested = format!("{flag}\nmod mutation_escape {{}}\n");
    assert_eq!(
        reservation_authority_shape::validate(authority, custody, staged, &nested),
        Err("authority-leaf-items")
    );
}

fn opened() -> String {
    let graph = CLOSED
        .replace(
            "mod transition_flag",
            "pub(in crate::mediator) mod transition_flag",
        )
        .replace(
            "mod staged_custody",
            "pub(in crate::mediator) mod staged_custody",
        )
        .replace("mod custody", "pub(in crate::mediator) mod custody")
        .replace("pub(super)", "pub(in crate::mediator)")
        .replace(
            "started: TransitionFlag,",
            "pub(in crate::mediator) started: TransitionFlag,",
        )
        .replace(
            "settled: TransitionFlag,",
            "pub(in crate::mediator) settled: TransitionFlag,",
        )
        .replace(
            "staged: StagedCustody,",
            "pub(in crate::mediator) staged: StagedCustody,",
        )
        .replace(
            "custody: AttemptCustody }",
            "pub(in crate::mediator) custody: AttemptCustody }",
        )
        .replace(
            "struct StagedCustody(RefCell",
            "struct StagedCustody(pub(in crate::mediator) RefCell",
        )
        .replace(
            "struct TransitionFlag(Cell",
            "struct TransitionFlag(pub(in crate::mediator) Cell",
        );
    graph.replace("// OPENED_EXTENSION", OPENED_EXTENSION)
        + "\n#[cfg(test)] mod opened_control { #[test] fn attacks_execute() { crate::mediator::exercise(); } }\n"
}

const OPENED_EXTENSION: &str = r#"
    pub fn exercise() {
        fn flag(run: fn(&reservation_state::AttemptReservation), settled: bool) {
            let attempt = reservation_state::fresh();
            run(&attempt);
            assert_eq!(attempt.custody.started.is_set(), !settled);
            assert_eq!(attempt.custody.settled.is_set(), settled);
        }
        fn staged(run: fn(&reservation_state::AttemptReservation)) {
            let attempt = reservation_state::fresh();
            assert!(!attempt.custody.staged.is_empty());
            run(&attempt);
            assert!(attempt.custody.staged.is_empty());
        }
        flag(reservation_state::descendant_attack::started, false);
        flag(reservation_state::descendant_attack::settled, true);
        flag(sibling_attack::started, false);
        flag(sibling_attack::settled, true);
        staged(sibling_attack::staged);
        let attempt = reservation_state::fresh();
        let mut future = Box::pin(reservation_state::descendant_attack::staged(&attempt));
        let waker = std::task::Waker::noop();
        let mut context = std::task::Context::from_waker(waker);
        assert!(matches!(std::future::Future::poll(future.as_mut(), &mut context), std::task::Poll::Ready(())));
        assert!(attempt.custody.staged.is_empty());
    }
"#;

fn prepare(scratch: &Path) {
    fs::write(
        scratch.join("Cargo.toml"),
        "[workspace]\n\n[package]\nname = \"reservation-authority-visibility\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lib]\npath = \"lib.rs\"\n",
    )
    .unwrap();
}

fn check(scratch: &Path) -> Output {
    cargo(scratch, &["check", "--offline", "--quiet"])
}

fn run(scratch: &Path) -> Output {
    cargo(scratch, &["test", "--offline", "--quiet"])
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
