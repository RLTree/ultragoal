use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use super::owned_compile_scratch::OwnedCompileScratch;

const CLOSED: &str = r#"
mod mediator {
    pub(crate) mod reservation_state {
        pub(crate) mod authority {
            use std::cell::{Cell, RefCell};
            pub(in crate::mediator) struct AttemptReservation {
                settled: Cell<bool>,
                staged: RefCell<Vec<()>>,
            }
        }
        pub(in crate::mediator) use authority::AttemptReservation;
        pub mod descendant_attack {
            use super::AttemptReservation;
            pub fn forge(a: &AttemptReservation) {
                a.settled.set(true);
                a.staged.borrow_mut().clear();
            }
        }
    }
    pub mod sibling_attack {
        use super::reservation_state::AttemptReservation;
        pub fn forge(a: &AttemptReservation) {
            a.settled.set(true);
            a.staged.borrow_mut().clear();
        }
    }
}
"#;

#[test]
fn sibling_and_descendant_cannot_mutate_reservation_storage() {
    assert_production_shape();
    let mut owned = OwnedCompileScratch::claim("routine-reservation-privacy");
    prepare(owned.path());

    fs::write(owned.path().join("lib.rs"), opened()).unwrap();
    let control = check(owned.path());
    assert!(control.status.success(), "{}", diagnostic(&control));

    fs::write(owned.path().join("lib.rs"), CLOSED).unwrap();
    let closed = check(owned.path());
    assert!(!closed.status.success(), "closed mutation probe compiled");
    let stderr = String::from_utf8_lossy(&closed.stderr);
    assert_eq!(stderr.matches("error[E0616]").count(), 4, "{stderr}");
    for field in ["settled", "staged"] {
        assert!(
            stderr.matches(&format!("field `{field}`")).count() >= 2,
            "{stderr}"
        );
    }
    owned.teardown_after_assertions();
}

fn assert_production_shape() {
    let graph =
        include_str!("../../src/routine_work/runtime_adapter/mediator/reservation_state/mod.rs");
    assert!(graph.contains("mod authority;"));
    assert!(graph.contains("pub(super) use authority::AttemptReservation;"));
    assert!(!graph.contains("reserved_test_attempt"));
    let source = include_str!(
        "../../src/routine_work/runtime_adapter/mediator/reservation_state/authority.rs"
    );
    let file = syn::parse_file(source).unwrap();
    assert!(
        !file
            .items
            .iter()
            .any(|item| matches!(item, syn::Item::Mod(_)))
    );
    let attempt = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Struct(item) if item.ident == "AttemptReservation" => Some(item),
            _ => None,
        })
        .expect("opaque reservation owner is declared");
    let syn::Visibility::Restricted(visibility) = &attempt.vis else {
        panic!("reservation owner visibility widened");
    };
    assert_eq!(
        visibility
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>(),
        ["super", "super"]
    );
    let fields = attempt
        .fields
        .iter()
        .map(|field| {
            assert!(matches!(field.vis, syn::Visibility::Inherited));
            field.ident.as_ref().unwrap().to_string()
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        fields,
        ["binding", "durable", "settled", "staged", "started"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
}

fn opened() -> String {
    CLOSED
        .replace(
            "settled: Cell<bool>",
            "pub(in crate::mediator) settled: Cell<bool>",
        )
        .replace(
            "staged: RefCell<Vec<()>>",
            "pub(in crate::mediator) staged: RefCell<Vec<()>>",
        )
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

fn diagnostic(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
