use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use super::owned_compile_scratch::OwnedCompileScratch;

#[path = "reservation_authority_shape.rs"]
mod reservation_authority_shape;

const CLOSED: &str = r#"
mod mediator {
    pub(crate) mod reservation_state {
        pub(crate) mod authority {
            use std::cell::{Cell, RefCell};
            pub(in crate::mediator) struct AttemptReservation {
                started: Cell<bool>,
                settled: Cell<bool>,
                staged: RefCell<Vec<()>>,
            }
        }
        pub(in crate::mediator) use authority::AttemptReservation;
        pub mod descendant_attack {
            use super::AttemptReservation;
            pub fn forge(a: &AttemptReservation) {
                a.started.set(true);
                a.settled.set(true);
                a.staged.borrow_mut().clear();
            }
        }
    }
    pub mod sibling_attack {
        use super::reservation_state::AttemptReservation;
        pub fn forge(a: &AttemptReservation) {
            a.started.set(true);
            a.settled.set(true);
            a.staged.borrow_mut().clear();
        }
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
    assert_eq!(stderr.matches("error[E0616]").count(), 6, "{stderr}");
    for field in ["started", "settled", "staged"] {
        assert!(
            stderr.matches(&format!("field `{field}`")).count() >= 2,
            "{stderr}"
        );
    }
    owned.teardown_after_assertions();
}

#[test]
fn build_specific_custody_side_effect_is_rejected() {
    let source = include_str!(
        "../../src/routine_work/runtime_adapter/mediator/reservation_state/authority.rs"
    );
    let staged = include_str!(
        "../../src/routine_work/runtime_adapter/mediator/reservation_state/staged_custody.rs"
    );
    let marker = "    pub(in super::super) fn protocol_id(&self) -> &String {\n        self.binding.protocol_id()\n    }";
    assert_eq!(source.matches(marker).count(), 1);
    let attributed = source.replacen(
        marker,
        "    #[cfg(test)]\n    pub(in super::super) fn protocol_id(&self) -> &String {\n        self.started.set(true);\n        self.binding.protocol_id()\n    }",
        1,
    );
    assert_eq!(
        reservation_authority_shape::validate(&attributed, staged),
        Err("authority-build-specific-or-hidden-code")
    );
    let side_effect = source.replacen(
        marker,
        "    pub(in super::super) fn protocol_id(&self) -> &String {\n        self.started.set(true);\n        self.binding.protocol_id()\n    }",
        1,
    );
    assert_eq!(
        reservation_authority_shape::validate(&side_effect, staged),
        Err("authority-custody-operations")
    );
}

fn opened() -> String {
    let graph = CLOSED
        .replace(
            "started: Cell<bool>",
            "pub(in crate::mediator) started: Cell<bool>",
        )
        .replace(
            "settled: Cell<bool>",
            "pub(in crate::mediator) settled: Cell<bool>",
        )
        .replace(
            "staged: RefCell<Vec<()>>",
            "pub(in crate::mediator) staged: RefCell<Vec<()>>",
        );
    graph.replace(
        "// OPENED_EXTENSION",
        "pub fn exercise() {\n        let attempt = reservation_state::authority::AttemptReservation {\n            started: std::cell::Cell::new(false),\n            settled: std::cell::Cell::new(false),\n            staged: std::cell::RefCell::new(vec![()]),\n        };\n        reservation_state::descendant_attack::forge(&attempt);\n        sibling_attack::forge(&attempt);\n        assert!(attempt.started.get());\n        assert!(attempt.settled.get());\n        assert!(attempt.staged.borrow().is_empty());\n    }",
    ) + "\n#[cfg(test)] mod opened_control {\n    #[test] fn both_attacks_execute() { crate::mediator::exercise(); }\n}\n"
}

fn prepare(scratch: &Path) {
    fs::write(
        scratch.join("Cargo.toml"),
        "[workspace]\n\n[package]\nname = \"reservation-authority-visibility\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lib]\npath = \"lib.rs\"\n",
    )
    .unwrap();
}

fn check(scratch: &Path) -> Output {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--offline", "--quiet"])
        .current_dir(scratch)
        .env("CARGO_TARGET_DIR", scratch.join("target"))
        .output()
        .unwrap()
}

fn run(scratch: &Path) -> Output {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["test", "--offline", "--quiet"])
        .current_dir(scratch)
        .env("CARGO_TARGET_DIR", scratch.join("target"))
        .output()
        .unwrap()
}

fn diagnostic(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
